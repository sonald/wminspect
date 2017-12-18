extern crate xcb;
extern crate xcb_util;
extern crate colored;
extern crate timer;
extern crate crossbeam;
extern crate libc;

use std;
use self::colored::*;
use std::fmt::*;
use std::time;
use xcb::xproto;
use std::sync::*;
use std::sync::atomic::{AtomicBool, self};
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

use super::filter::*;

/// helper type to format vec of window
struct HexedVec<'a, T: 'a>(&'a Vec<T>);

impl<'a, T: Debug + LowerHex> Debug for HexedVec<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut has_next = false;
        let mut s = String::new();
        write!(&mut s, "[")?;
        for t in self.0 {
            let prefix = if has_next { ", " } else { "" };
            write!(&mut s, "{}{:#x}", prefix, t)?;
            has_next = true;
        }
        write!(&mut s, "]")?;

        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Display for Geometry {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", format!("{}x{}+{}+{}", self.width, self.height,
                                self.x, self.y))
    }
}

impl Geometry {
    pub fn update_with_configure(&mut self, cne: &xcb::ConfigureNotifyEvent) {
        self.x = cne.x();
        self.y = cne.y();
        self.width = cne.width();
        self.height = cne.height();
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq)]
pub enum MapState {
    Unmapped,
    Viewable,
    Unviewable,
}

impl PartialEq for MapState {
    fn eq(&self, other: &Self) -> bool {
        return (*self as i32) == (*other as i32);
    }
}

impl Display for MapState {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", match *self {
            MapState::Unmapped => "Unmapped",
            MapState::Unviewable => "Unviewable",
            MapState::Viewable => "Viewable"
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    pub override_redirect: bool,
    pub map_state: MapState,
}

impl Display for Attributes {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}{}", if self.override_redirect { "OR " } else {""}, 
               self.map_state)
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: xcb::Window,
    pub name: String,
    pub attrs: Attributes,
    pub geom: Geometry,
    valid: bool,
}

impl Eq for Window {}

impl Ord for Window {
    fn cmp(&self, other: &Window) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Window {
    fn partial_cmp(&self, other: &Window) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.id == other.id
    }
}

impl Display for Window {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let id = format!("0x{:x}", self.id);
        write!(f, "{}({}) {} {}", id, self.name, self.geom, self.attrs)
    }
}

impl Window {
    fn is_window_pinned(&self, filter: &Filter) -> bool {
        for rule in &filter.rules {
            if rule.action == Action::Pin && rule.func.as_ref()(self) {
                return true;
            }
        }

        false
    }
}

type WindowStackView = Vec<xcb::Window>;
type WindowListView = HashSet<xcb::Window>;

/// contains cached windows data, which should keep in sync with server
struct WindowsLayout {
    /// collected window infos
    windows: HashMap<xcb::Window, Window>,
    /// a view maintained by stacking order (bottom -> top)
    stack_view: WindowStackView,

    filtered_view: WindowStackView,
    pinned_windows: WindowListView,
}

#[derive(Debug, Clone)]
pub enum Condition {
    Colorful,
    MappedOnly,
    OmitHidden,
    NoSpecial,
    ShowDiff,
    ClientsOnly,
}

pub struct Context<'a> {
    pub c: &'a xcb_util::ewmh::Connection,
    pub root: xcb::Window,
    filter: Mutex<Filter>,

    pub options: Vec<Condition>,

    //TODO: move into inner struct as one, and save two extra locks
    inner: Mutex<WindowsLayout>,
    
}

pub enum XcbRequest<'a> {
    GWA(xcb::GetWindowAttributesCookie<'a>),
    GE(xcb::GetGeometryCookie<'a>),
    GP(xcb::GetPropertyCookie<'a>),
}

#[derive(Clone)]
pub enum Message {
    LastConfigureEvent(xcb::ffi::xcb_configure_notify_event_t),
    Reset,
    Quit,
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter) -> Result {
        use self::Message::*;
        match self {
            &LastConfigureEvent(ref raw) => {
                write!(f, "Message::LastConfigureEvent(ConfigureNotify{{\
                    w: {:#x}, above: {:#x}, x: {:#x}, y: {:#x}, width: {:#x}, height: {:#x}}})",
                    raw.window, raw.above_sibling, raw.x, raw.y, raw.width, raw.height)
            },
            &Reset => write!(f, "Message::Reset"),
            &Quit => write!(f, "Message::Quit"),
        }
    }
}

fn as_event<'r, T>(e: &'r xcb::GenericEvent) -> &'r T {
    return unsafe { xcb::cast_event::<T>(&e) };
}

macro_rules! build_fun {
    ($getter:ident, $setter:ident, $cond:tt) => (
        pub fn $getter(&self) -> bool {
            self.options.as_slice().iter().any(|c| {
                match *c {
                    Condition::$cond => true,
                    _ => false
                }
            })
        }
        
        pub fn $setter(&mut self) {
            self.options.push(Condition::$cond)
        })
}


impl<'a> Context<'a> {
    build_fun!(mapped_only, set_mapped_only, MappedOnly);
    build_fun!(colorful, set_colorful, Colorful);
    build_fun!(omit_hidden, set_omit_hidden, OmitHidden);
    build_fun!(no_special, set_no_special, NoSpecial);
    build_fun!(show_diff, set_show_diff, ShowDiff);
    build_fun!(clients_only, set_clients_only, ClientsOnly);

    pub fn new(c: &'a xcb_util::ewmh::Connection, f: Filter) -> Context<'a> {
        let screen = c.get_setup().roots().next().unwrap();

        Context {
            c: c,
            root: screen.root(),
            filter: Mutex::new(f),
            options: Vec::new(),

            inner: Mutex::new(
                WindowsLayout {
                    windows:  HashMap::new(),
                    stack_view: WindowStackView::new(),

                    filtered_view: WindowStackView::new(),
                    pinned_windows: WindowListView::new(),
                })
        }
    }

    /// `changes` is updated windows for current event
    /// TODO: highlight pinned windows in different style
    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let layout = self.inner.lock().unwrap();

        let colored = self.colorful();
        for (i, wid) in layout.filtered_view.iter().enumerate() {
            let w = layout.windows.get(wid).expect(&format!("{} does not exist!", wid));

            if self.show_diff() && changes.is_some() &&
                changes.as_ref().unwrap().contains(&wid) {
                println!("{}: {}", i, win2str(w, colored).on_white());
            } else {
                println!("{}: {}", i, win2str(w, colored));
            }
        }
    }

    /// Tell if window is contained in current filter rule set.
    pub fn is_window_concerned(&self, w: xcb::Window) -> bool {
        let layout = self.inner.lock().unwrap();
        layout.filtered_view.iter().any(|&id| id == w)
    }

    /// add Window to the stack
    pub fn update_with(&self, w: Window) {
        let wid = w.id;

        let mut layout = self.inner.lock().unwrap();
        let filter = self.filter.lock().unwrap();

        layout.stack_view.push(wid);
        if filter.apply_to(&w) {
            layout.filtered_view.push(wid);
            wm_debug!("filtered_view {:?}", HexedVec(&layout.filtered_view));
        }
        if w.is_window_pinned(&filter) {
            layout.pinned_windows.insert(wid);
        }
        layout.windows.entry(w.id).or_insert(w);
    }

    pub fn update_pin_state(&self, wid: xcb::Window) {
        let mut layout = self.inner.lock().unwrap();
        let filter = self.filter.lock().unwrap();

        let pinned = if let Some(win) = layout.windows.get_mut(&wid) {
            win.is_window_pinned(&filter)
        } else {
            return;
        };

        if pinned {
            layout.pinned_windows.insert(wid);
        } else {
            layout.pinned_windows.remove(&wid);
        }
    }

    pub fn remove(&self, wid: xcb::Window) {
        let mut layout = self.inner.lock().unwrap();
        layout.windows.remove(&wid);
        layout.stack_view.retain(|&w| w != wid);
        layout.filtered_view.retain(|&w| w != wid);
        layout.pinned_windows.retain(|&w| w != wid);
    }

    /// lock and call `f`, do not call any locking operations in `f`
    pub fn with_window_mut<F>(&self, wid: xcb::Window, mut f: F) where F: FnMut(&mut Window) {
        let mut layout = self.inner.lock().unwrap();
        if let Some(win) = layout.windows.get_mut(&wid) {
            f(win);
        } else {
            wm_debug!("with_window_mut: bad wid {}", wid);
        }
    }


    /// refresh internal windows cache from xserver
    /// this is a very heavy operation and may stop the world now
    /// (may be moved into a thread or so)
    pub fn refresh_windows(&self) {

        let mut layout = self.inner.lock().unwrap();
        let windows = self.collect_windows();

        layout.stack_view = windows.iter().map(|w| w.id).collect();
        layout.pinned_windows = self.collect_pinned_windows(&windows);
        layout.windows = windows.clone().into_iter().map(|w| (w.id, w)).collect();

        self.rebuild_filter();
        layout.filtered_view = self.apply_filter(&windows);

        //wm_debug!("stack_view: {:?}, \nfiltered_view: {:?}",
                  //HexedVec(&layout.stack_view), HexedVec(&layout.filtered_view));
    }

    fn update_stack_unlocked(&self, layout: &mut WindowsLayout, wid: xcb::Window, above: xcb::Window) {
        //wm_debug!("update_stack_unlocked {:#x} {:#x}", wid, above);
        if !layout.windows.contains_key(&wid) {
            return;
        }

        layout.stack_view.retain(|&w| w != wid);
        if above == xcb::WINDOW_NONE {
            layout.stack_view.insert(0, wid);
        } else {
            //TODO: check if operation needed
            let idx = layout.stack_view.iter().position(|&x| x == above).unwrap();
            layout.stack_view.insert(idx+1, wid);
        }

        if layout.filtered_view.iter().any(|&id| id == wid) {
            wm_debug!("update_stack_unlocked {:#x} {:#x}", wid, above);
            //wm_debug!("PRE: filtered_view: {:?}", HexedVec(&layout.filtered_view));
            layout.filtered_view.retain(|&w| w != wid);
            if above == xcb::WINDOW_NONE || layout.filtered_view.len() == 0 {
                layout.filtered_view.insert(0, wid);
            } else {
                if let Some(idx) = layout.filtered_view.iter().position(|&x| x == above) {
                    layout.filtered_view.insert(idx+1, wid);
                } else {
                    // find neareast lower sibling as above_sibling
                    let lower_id = *layout.stack_view.iter().rev().skip_while(|&&id| id == above)
                        .find(|&&w| layout.filtered_view.iter().position(|&id| id == w).is_some())
                        .unwrap();
                    let upper_bound = layout.filtered_view.iter().position(|&w| w == lower_id).unwrap();
                    layout.filtered_view.insert(upper_bound+1, wid);
                }
            }
            //wm_debug!("POST: filtered_view: {:?}", HexedVec(&layout.filtered_view));
        }
    }

    /// sync stack from configure notify
    pub fn update_stack(&self, wid: xcb::Window, above: xcb::Window) {
        let mut layout = self.inner.lock().unwrap();
        self.update_stack_unlocked(&mut layout, wid, above);
    }

    fn update_window_unlocked(&self, layout: &mut WindowsLayout, cne: &xcb::ConfigureNotifyEvent) {
        let wid = cne.window();

        if !layout.windows.contains_key(&wid) {
            return;
        }

        if let Some(win) = layout.windows.get_mut(&wid) {
            win.geom.update_with_configure(cne);
        }
    }

    /// update inner window layout from configure event
    pub fn update_window(&self, cne: &xcb::ConfigureNotifyEvent) {
        //wm_debug!("update_window {:#x} ", cne.window());
        let mut layout = self.inner.lock().unwrap();
        let wid = cne.window();

        self.update_stack_unlocked(&mut layout, wid, cne.above_sibling());
        self.update_window_unlocked(&mut layout, cne);
    }

    fn collect_pinned_windows(&self, windows: &Vec<Window>) -> WindowListView {
        let filter = self.filter.lock().unwrap();
        let f = |w| {
            for rule in &filter.rules {
                if rule.action == Action::Pin && rule.func.as_ref()(w) {
                    return Some(w.id.clone());
                }
            }
            None
        };

        windows.iter().filter_map(f).collect()
    }


    fn collect_windows(&self) ->Vec<Window> {
        let c = self.c;

        let res = match xcb::query_tree(&c, self.root).get_reply() {
            Ok(result) => result,
            Err(_) => return Vec::new(),
        };

        let target_windows = self.query_windows(&res);
        wm_debug!("initial total #{}", target_windows.len());
        target_windows
    }

    /// rebuild filter rule set
    /// rebuild will clear all adhoc rules and readd them for now, since some 
    /// conditions can be changed (e.g _NET_CLIENT_LIST_STACKING)
    fn rebuild_filter(&self) {
        macro_rules! adhoc {
            ($filter:ident, $c:expr) => ({
                let afp = ActionFuncPair {
                    action: Action::FilterOut,
                    rule: FilterRule::Adhoc,
                    func: Box::new( $c )
                };
                $filter.add_live_rule(afp);
            })
        }

        let mut filter = self.filter.lock().unwrap();

        if self.mapped_only() || self.omit_hidden() {
            // TODO: rewrite with _NET_WM_STATE of window
            /*
            let has_guard_window = target_windows.iter()
                .any(|w| w.name.contains("guard window") && w.attrs.override_redirect);

            if has_guard_window {
                wm_debug!("has guard window, filter out not mapped or hidden");
            }

            do_filter!(target_windows, skip_while, |w| {
                if has_guard_window {
                    !w.name.contains("guard window") || !w.attrs.override_redirect
                } else {
                    false
                }
            });
            */

            if self.mapped_only() {
                adhoc!(filter, |w| { w.attrs.map_state == MapState::Viewable });
            }

            if self.omit_hidden() {
                let (screen_width, screen_height) = {
                    let screen = self.c.get_setup().roots().next().unwrap();
                    (screen.width_in_pixels(), screen.height_in_pixels())
                };

                adhoc!(filter, move |w| {
                    w.geom.x < screen_width as i16 &&
                        w.geom.y < screen_height as i16 &&
                        (w.geom.width as i16) + w.geom.x > 0 && (w.geom.height as i16) + w.geom.y > 0
                });
            }
        }

        if self.no_special() {
            let specials = hashset!(
                ("mutter guard window"),
                ("deepin-metacity guard window"),
                ("mutter topleft corner window"),
                ("deepin-metacity topleft corner window"),
                );

            adhoc!(filter, move |w| !specials.contains(w.name.as_str()));
        }

        if self.clients_only() {
            self.update_clients_only_rule_locked(&mut filter);
        }
    }

    fn update_clients_only_rule_locked(&self, filter: &mut Filter) {
        //NOTE: clients is changing overtime
        //dont figure out how to solve it, so we need to re-build this 
        //rule on the air every time clients list gets updated.
        //or make boxed closure's lifetime as long as filter instead of static
        let clients = self.collect_window_manager_properties();

        if let Some(i) = filter.rules.iter().position(|r| r.rule == FilterRule::ClientsOnly) {
            let r = filter.rules.get_mut(i).unwrap();
            r.func = Box::new(move |w| clients.contains(&w.id));
        } else {
            let afp = ActionFuncPair {
                action: Action::FilterOut,
                rule: FilterRule::ClientsOnly,
                func: Box::new(move |w| clients.contains(&w.id))
            };
            filter.rules.push(afp);
        }
    }

    pub fn update_clients(&self) {
        let mut layout = self.inner.lock().unwrap();
        let mut filter = self.filter.lock().unwrap();

        //self.rebuild_filter();
        self.update_clients_only_rule_locked(&mut filter);

        layout.filtered_view = layout.windows.iter()
            .filter(|&(_, w)| filter.apply_to(w)).map(|(_, w)| w.id).collect();
    }

    /// filter windows by applying loaded rules
    fn apply_filter(&self, windows: &Vec<Window>) -> WindowStackView {
        let filter = self.filter.lock().unwrap();
        windows.iter().filter(|w| filter.apply_to(w)).map(|w| w.id).collect()
    }

    fn collect_window_manager_properties(&self) -> WindowStackView {
        let c = self.c;

        let cookie = xcb_util::ewmh::get_client_list_unchecked(&c, 0);
        match cookie.get_reply() {
            Ok(ref reply) => {
                    let list = reply.windows().to_vec();
                    wm_debug!("CLIENT_LIST: {:#?}", HexedVec(&list));
                    list
            },
            _ => Vec::new()
        }
    }

    pub fn query_window(&self, id: xcb::Window) -> Window {
        let c = self.c;

        let mut qs: Vec<XcbRequest> = Vec::new();
        qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, id)));
        qs.push(XcbRequest::GE(xcb::get_geometry(&c, id)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, id, self.c.WM_NAME(),
                                                 unsafe{&*c.get_raw_conn()}.UTF8_STRING, 0, std::u32::MAX)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, id, xcb::xproto::ATOM_WM_NAME, 
                                                 xcb::xproto::ATOM_STRING, 0, std::u32::MAX)));

        macro_rules! apply_reply {
            ($win:ident $cookie:ident $reply:ident $e:expr) => (
                match $cookie.get_reply() {
                    Ok($reply) => $e,
                    Err(_) => $win.valid = false,
                })
        }

        let mut win = Window {
            id: id,
            name: "".to_string(),
            attrs: Attributes{override_redirect: false, map_state: MapState::Unmapped},
            geom: Geometry{x:0,y:0,width:0,height:0},
            valid: true,
        };

        for query in qs {
            match query {
                XcbRequest::GWA(cookie) => {
                    apply_reply!(win cookie reply {
                        win.attrs.override_redirect = reply.override_redirect();
                        win.attrs.map_state = match reply.map_state() {
                            0 => MapState::Unmapped,
                            2 => MapState::Viewable,
                            _ => MapState::Unviewable,
                        };
                    })
                },
                XcbRequest::GE(cookie) => {
                    apply_reply!(win cookie reply {
                        win.geom = Geometry {
                            x: reply.x(), 
                            y: reply.y(),
                            width: reply.width(),
                            height: reply.height(),
                        };
                    })
                },
                XcbRequest::GP(cookie) => {
                    apply_reply!(win cookie reply {
                        if reply.value_len() > 0 && win.name.len() == 0 {
                            win.name = String::from_utf8(reply.value::<u8>().to_vec()).unwrap_or("".to_string());
                        }
                    })
                },
            }
        }

        return win;
    }

    fn query_windows(&self, res: &xcb::QueryTreeReply) -> Vec<Window> {
        let c = self.c;

        let mut qs: Vec<XcbRequest> = Vec::new();
        for w in res.children() {
            qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, *w)));
            qs.push(XcbRequest::GE(xcb::get_geometry(&c, *w)));
            qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, c.WM_NAME(),
                                                    unsafe{&*c.get_raw_conn()}.UTF8_STRING, 0, std::u32::MAX)));
            qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, xcb::xproto::ATOM_WM_NAME, 
                                                     xcb::xproto::ATOM_STRING, 0, std::u32::MAX)));
        }

        macro_rules! apply_reply {
            ($win:ident $cookie:ident $reply:ident $e:expr) => (
                match $cookie.get_reply() {
                    Ok($reply) => $e,
                    Err(_) => $win.valid = false,
                })
        }

        let mut windows = Vec::with_capacity(res.children_len() as usize);
        let window_ids = res.children();
        for (i, query) in qs.into_iter().enumerate() {
            let idx = i / 4;
            if i % 4 == 0 {
                windows.push(Window {
                    id: window_ids[idx],
                    name: "".to_string(),
                    attrs: Attributes{override_redirect: false, map_state: MapState::Unmapped},
                    geom: Geometry{x:0,y:0,width:0,height:0},
                    valid: true,
                });
            }

            if let Some(win) = windows.last_mut() {
                match query {
                    XcbRequest::GWA(cookie) => {
                        apply_reply!(win cookie reply {
                            win.attrs.override_redirect = reply.override_redirect();
                            win.attrs.map_state = match reply.map_state() {
                                0 => MapState::Unmapped,
                                2 => MapState::Viewable,
                                _ => MapState::Unviewable,
                            };
                        })
                    },
                    XcbRequest::GE(cookie) => {
                        apply_reply!(win cookie reply {
                            win.geom = Geometry {
                                x: reply.x(), 
                                y: reply.y(),
                                width: reply.width(),
                                height: reply.height(),
                            };
                        })
                    },
                    XcbRequest::GP(cookie) => {
                        apply_reply!(win cookie reply {
                            if reply.value_len() > 0 && win.name.len() == 0 {
                                win.name = String::from_utf8(reply.value::<u8>().to_vec()).unwrap_or("".to_string());
                            }
                        })
                    },
                }
            }

        }

        return windows;
    }
}


pub fn monitor(ctx: &Context) {
    let ev_mask: u32 = xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY | xproto::EVENT_MASK_PROPERTY_CHANGE;
    xcb::xproto::change_window_attributes(&ctx.c, ctx.root,
                                          &[(xcb::xproto::CW_EVENT_MASK, ev_mask)]);
    ctx.c.flush();

    ctx.refresh_windows();

    let need_configure = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<Message>();

    ctx.dump_windows(None);

    crossbeam::scope(|scope| {
        {
            scope.spawn(move || {
                let idle_configure_timeout = time::Duration::from_millis(50);
                let mut last_checked_time = time::Instant::now();

                let mut raw_cne = None;

                loop {
                    match rx.recv_timeout(time::Duration::from_millis(10)) {
                        Ok(Message::LastConfigureEvent(raw)) => { 
                            last_checked_time = time::Instant::now();
                            need_configure.store(true, atomic::Ordering::Release);
                            raw_cne = Some(raw);
                        },
                        Ok(Message::Reset) => { 
                            need_configure.store(false, atomic::Ordering::Release);
                        },
                        Ok(Message::Quit) => { break; },
                        _ =>  {}
                    }

                    if need_configure.load(atomic::Ordering::Acquire) && last_checked_time.elapsed() > idle_configure_timeout {
                        let raw_cne = raw_cne.unwrap();
                        let cne = xcb::ConfigureNotifyEvent::new(
                            raw_cne.event, raw_cne.window, raw_cne.above_sibling,
                            raw_cne.x, raw_cne.y, raw_cne.width, raw_cne.height, 
                            raw_cne.border_width,
                            if raw_cne.override_redirect == 0 {false} else {true});

                        if ctx.is_window_concerned(cne.window()) {
                            wm_debug!("timedout, reload");
                            println!("delayed configure {:#x} ", cne.window());

                            let diff = if ctx.show_diff() {
                                Some(hashset!(cne.window(), cne.above_sibling()))
                            } else {
                                None
                            };
                            
                            ctx.dump_windows(diff);
                            need_configure.store(false, atomic::Ordering::Release);
                        }
                    }
                }

            });
        }


        let mut last_configure_xid = xcb::WINDOW_NONE;
        let mut last_event_type = 0;
        loop {
            if let Some(ev) = ctx.c.wait_for_event() {
                //wm_debug!("event: {}", ev.response_type() & !0x80);
                match ev.response_type() & !0x80 {
                    xcb::xproto::CREATE_NOTIFY => {
                        let cne = as_event::<xcb::CreateNotifyEvent>(&ev);
                        if cne.parent() != ctx.root {
                            break;
                        }
                        println!("create 0x{:x}, parent 0x{:x}", cne.window(), cne.parent());

                        // assumes that window will be at top when created
                        let new_win = ctx.query_window(cne.window());
                        ctx.update_with(new_win);
                        let diff = if ctx.show_diff() {
                            Some(hashset!(cne.window()))
                        } else {
                            None
                        };

                        ctx.dump_windows(diff);
                    },
                    xcb::xproto::DESTROY_NOTIFY => {
                        let dne = as_event::<xcb::DestroyNotifyEvent>(&ev);

                        if ctx.is_window_concerned(dne.window()) {
                            println!("destroy 0x{:x}", dne.window());
                            ctx.remove(dne.window());

                            ctx.dump_windows(None);
                        }
                    },

                    xcb::xproto::REPARENT_NOTIFY => {
                        let rne = as_event::<xcb::ReparentNotifyEvent>(&ev);

                        if ctx.is_window_concerned(rne.window()) {
                            if rne.parent() != ctx.root {
                                println!("reparent 0x{:x} to 0x{:x}", rne.window(), rne.parent());
                                ctx.remove(rne.window());

                                ctx.dump_windows(None);

                            } else {
                                println!("reparent 0x{:x} to root", rne.window());
                                let new_win = ctx.query_window(rne.window());
                                ctx.update_with(new_win);

                                let diff = if ctx.show_diff() {
                                    Some(hashset!(rne.window()))
                                } else {
                                    None
                                };
                                ctx.dump_windows(diff);
                            }
                        }
                    },

                    xproto::CONFIGURE_NOTIFY => {
                        let cne = as_event::<xcb::ConfigureNotifyEvent>(&ev);
                        ctx.update_window(cne);

                        if ctx.is_window_concerned(cne.window()) {
                            if last_configure_xid != cne.window() {
                                println!("configure 0x{:x} above: 0x{:x}", cne.window(), cne.above_sibling());
                                let diff = if ctx.show_diff() {
                                    Some(hashset!(cne.window(), cne.above_sibling()))
                                } else {
                                    None
                                };


                                ctx.dump_windows(diff);
                                last_configure_xid = cne.window();
                                tx.send(Message::Reset).unwrap();

                            } else {
                                let clone: xcb::ffi::xcb_configure_notify_event_t = unsafe {*cne.ptr}.clone();
                                tx.send(Message::LastConfigureEvent(clone)).unwrap();
                            }
                        }
                    },

                    xproto::MAP_NOTIFY => {
                        let mn = as_event::<xcb::MapNotifyEvent>(&ev);

                        if ctx.is_window_concerned(mn.window()) {
                            ctx.with_window_mut(mn.window(), |win| {
                                win.attrs.map_state = MapState::Viewable;
                            });
                            ctx.update_pin_state(mn.window());

                            println!("map 0x{:x}", mn.window());

                            let diff = if ctx.show_diff() {
                                Some(hashset!(mn.window()))
                            } else {
                                None
                            };
                            ctx.dump_windows(diff);
                        }
                    },

                    xproto::UNMAP_NOTIFY => {
                        let un = as_event::<xcb::UnmapNotifyEvent>(&ev);

                        if ctx.is_window_concerned(un.window()) {
                            ctx.with_window_mut(un.window(), |win| {
                                win.attrs.map_state = MapState::Unmapped;
                            });
                            ctx.update_pin_state(un.window());
                            println!("unmap 0x{:x}", un.window());
                            ctx.dump_windows(None);
                        }
                    },

                    xproto::PROPERTY_NOTIFY => {
                        let pn = as_event::<xcb::PropertyNotifyEvent>(&ev);
                        if pn.window() == ctx.root {
                            if pn.atom() == ctx.c.CLIENT_LIST_STACKING() {
                                if last_event_type == xproto::CREATE_NOTIFY ||
                                    last_event_type == xproto::DESTROY_NOTIFY {
                                    ctx.update_clients();
                                    ctx.dump_windows(None);
                                }
                            }
                        } else {
                            wm_debug!("prop for {:#x}", pn.window());
                        }
                    },

                    _ => {
                    },
                } 

                last_event_type = ev.response_type() & !0x80;
            };
        }

        match tx.send(Message::Quit) {
            Ok(_) => {},
            Err(_) => {wm_debug!("send message error")}
        }
    });
}

fn get_tty_cols() -> Option<usize> {
    let mut winsz: libc::winsize;
    unsafe {
        winsz = std::mem::uninitialized();
        match libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, 
                          &mut winsz as *mut libc::winsize) {
            0 => Some(winsz.ws_col as usize),
            _ => None
        }
    }
}

//TODO: cut off name according to tty columns
fn win2str(w: &Window, mut colored: bool) -> String {
    let geom_str = format!("{}", w.geom);
    let id = format!("0x{:x}", w.id);
    let attrs = format!("{}", w.attrs);

    if unsafe { libc::isatty(libc::STDOUT_FILENO) } == 0 {
        colored = false;
    } 
    let cols = get_tty_cols().unwrap_or(80) / 2;
    //FIXME: try estimate length by bytes, not chars
    let name = w.name.chars().take(cols).collect::<String>();

    if colored {
        format!("{}({}) {} {}", id.blue(), name.cyan(), geom_str.red(), attrs.green())
    } else {
        format!("{}({}) {} {}", id, w.name, geom_str, attrs)
    }
}

