extern crate xcb;
extern crate colored;
extern crate timer;
extern crate crossbeam;
extern crate libc;

use std;
use self::colored::*;
use std::fmt::*;
use std::time;
use std::thread;
use xcb::xproto;
use std::sync::*;
use std::sync::atomic::{AtomicBool, self};
use std::collections::{HashMap, HashSet};
use std::cmp::Ordering;

use super::filter::*;
use super::macros::print_type_of;

#[derive(Debug, Clone, Copy)]
pub struct Geometry<T> where T: Copy {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> Display for Geometry<T> where T: Display + Copy {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", format!("{}x{}+{}+{}", self.width, self.height,
                                self.x, self.y))
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
    pub geom: Geometry<i32>,
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

#[derive(Clone)]
pub struct Context<'a, 'b> {
    pub c: &'a xcb::Connection,
    pub filter: &'b Filter,

    /// atom caches
    net_wm_name_atom: xcb::Atom,
    utf8_string_atom: xcb::Atom,

    /// collected window infos
    windows: Arc<Mutex<HashMap<xcb::Window, Window>>>,
    /// a view sorted by stacking order (bottom -> top)
    stack_view: Arc<Mutex<WindowStackView>>,
    pinned_windows: Arc<Mutex<WindowListView>>,

}

pub enum XcbRequest<'a> {
    GWA(xcb::GetWindowAttributesCookie<'a>),
    GE(xcb::GetGeometryCookie<'a>),
    GP(xcb::GetPropertyCookie<'a>),
}

macro_rules! do_filter {
    ($windows:ident, $op:ident, $rule:expr) => (
        $windows = $windows.into_iter(). $op ( $rule ) .collect::<Vec<_>>();
        )
}

#[derive(Debug, Clone)]
pub enum Message {
    TimeoutEvent,
    Reset,
    Quit,
}

fn as_event<'r, T>(e: &'r xcb::GenericEvent) -> &'r T {
    return unsafe { xcb::cast_event::<T>(&e) };
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn new(c: &'a xcb::Connection, f: &'b Filter) -> Context<'a, 'b> {
        Context {
            c: c,
            filter: f,
            net_wm_name_atom: xcb::intern_atom(c, false, "_NET_WM_NAME").get_reply().unwrap().atom(),
            utf8_string_atom: xcb::intern_atom(c, false, "UTF8_STRING").get_reply().unwrap().atom(),

            windows: Arc::new(Mutex::new(HashMap::new())),
            stack_view: Arc::new(Mutex::new(WindowStackView::new())),
            pinned_windows: Arc::new(Mutex::new(WindowListView::new())),
        }
    }

    /// `changes` is updated windows for current event
    /// TODO: highlight pinned windows in different style
    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let windows = self.windows.lock().unwrap();

        let colored = self.filter.colorful();
        for (i, wid) in self.stack_view.lock().unwrap().iter().enumerate() {
            let w = windows.get(wid).expect(&format!("{} does not exist!", wid));

            if self.filter.show_diff() && changes.is_some() &&
                changes.as_ref().unwrap().contains(&wid) {
                println!("{}: {}", i, win2str(w, colored).on_white());
            } else {
                println!("{}: {}", i, win2str(w, colored));
            }
        }
    }

    pub fn is_window_concerned(&self, w: xcb::Window) -> bool {
        self.windows.lock().unwrap().contains_key(&w)
    }

    /// add Window to the stack
    pub fn update_with(&self, w: Window) {
        let mut windows = self.windows.lock().unwrap();
        let w = windows.entry(w.id).or_insert(w);
        self.stack_view.lock().unwrap().push(w.id);
        if w.is_window_pinned(self.filter) {
            self.pinned_windows.lock().unwrap().insert(w.id);
        }
    }

    pub fn update_pin_state(&self, w: &Window) {
        if w.is_window_pinned(self.filter) {
            self.pinned_windows.lock().unwrap().insert(w.id);
        } else {
            self.pinned_windows.lock().unwrap().remove(&w.id);
        }
    }

    pub fn remove(&self, wid: xcb::Window) {
        self.windows.lock().unwrap().remove(&wid);
        self.stack_view.lock().unwrap().retain(|&w| w == wid);
        self.pinned_windows.lock().unwrap().retain(|&w| w == wid);
    }

    pub fn get_windows(&self) -> MutexGuard<HashMap<xcb::Window, Window>> {
        self.windows.lock().unwrap()
    }

    //pub fn get_window_mut(&self, wid: xcb::Window) -> Option<&mut Window> {
        //let mut windows = self.windows.lock().unwrap();
        //windows.get_mut(&wid)
    //}


    /// refresh internal windows cache from xserver
    /// this is a very heavy operation and may stop the world now
    /// (may be moved into a thread or so)
    pub fn refresh_windows(&self) {
        let windows = self.collect_windows();
        {
            let mut lock = self.stack_view.lock().unwrap();
            *lock = windows.iter().map(|w| w.id).collect();
        }

        {
            let mut lock = self.pinned_windows.lock().unwrap();
            *lock = self.collect_pinned_windows();
        }

        {
            let mut lock = self.windows.lock().unwrap();
            *lock = windows.into_iter().map(|w| (w.id, w)).collect();
        }
    }

    fn collect_pinned_windows(&self) -> WindowListView {
        let filter = self.filter;
        let f = |(_, w): (&xcb::Window, &Window)| {
            for rule in &filter.rules {
                if rule.action == Action::Pin && rule.func.as_ref()(w) {
                    return Some(w.id.clone());
                }
            }
            None
        };

        self.windows.lock().unwrap().iter().filter_map(f).collect()
    }


    pub fn collect_windows(&self) ->Vec<Window> {
        let c = self.c;

        let screen = c.get_setup().roots().next().unwrap();
        let res = match xcb::query_tree(&c, screen.root()).get_reply() {
            Ok(result) => result,
            Err(_) => return Vec::new(),
        };

        let mut target_windows = self.query_windows(&res);
        wm_debug!("initial total #{}", target_windows.len());

        if self.filter.mapped_only() || self.filter.omit_hidden() {
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

            if self.filter.mapped_only() {
                do_filter!(target_windows, filter, |w| { w.attrs.map_state == MapState::Viewable });
            }

            if self.filter.omit_hidden() {
                do_filter!(target_windows, filter, |w| {
                    w.geom.x < screen.width_in_pixels() as i32 && w.geom.y < screen.height_in_pixels() as i32 &&
                        w.geom.width + w.geom.x > 0 && w.geom.height + w.geom.y > 0
                });
            }
        }

        if self.filter.no_special() {
            let specials = hashset!(
                ("mutter guard window"),
                ("deepin-metacity guard window"),
                ("mutter topleft corner window"),
                ("deepin-metacity topleft corner window"),
                );
            do_filter!(target_windows, filter, |w: &Window| { !specials.contains(w.name.as_str()) });
        }

        if self.filter.rules.len() > 0 {
            for rule in &self.filter.rules {
                if rule.action == Action::FilterOut {
                    do_filter!(target_windows, filter, rule.func.as_ref());
                }
            }
        }

        return target_windows;
    }


    pub fn query_window(&self, id: xcb::Window) -> Window {
        let c = self.c;

        let mut qs: Vec<XcbRequest> = Vec::new();
        qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, id)));
        qs.push(XcbRequest::GE(xcb::get_geometry(&c, id)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, id, self.net_wm_name_atom,
        self.utf8_string_atom, 0, std::u32::MAX)));
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
                            x: reply.x() as i32, 
                            y: reply.y() as i32,
                            width: reply.width() as i32,
                            height: reply.height() as i32,
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
            qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, self.net_wm_name_atom,
                self.utf8_string_atom, 0, std::u32::MAX)));
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
                                x: reply.x() as i32, 
                                y: reply.y() as i32,
                                width: reply.width() as i32,
                                height: reply.height() as i32,
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


pub fn monitor(c: &xcb::Connection, screen: &xcb::Screen, filter: &Filter) {
    let ctx = Arc::new(Context::new(c, filter));

    let ev_mask: u32 = xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY;
    xcb::xproto::change_window_attributes(ctx.c, screen.root(), 
                                          &[(xcb::xproto::CW_EVENT_MASK, ev_mask)]);
    c.flush();

    ctx.refresh_windows();

    let last_configure_xid = Arc::new(Mutex::new(xcb::WINDOW_NONE));
    let need_configure = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<Message>();

    ctx.dump_windows(None);

    crossbeam::scope(|scope| {

        {
            let ctx = ctx.clone();
            let last_configure_xid = last_configure_xid.clone();

            scope.spawn(move || {
                let idle_configure_timeout = time::Duration::from_millis(200);
                let mut last_checked_time = time::Instant::now();

                loop {
                    match rx.recv_timeout(time::Duration::from_millis(10)) {
                        Ok(Message::TimeoutEvent) => { 
                            wm_debug!("recv timeout"); 
                            last_checked_time = time::Instant::now();
                            need_configure.store(true, atomic::Ordering::Release)
                        },
                        Ok(Message::Reset) => { 
                            need_configure.store(false, atomic::Ordering::Release)
                        },
                        Ok(Message::Quit) => { break; },
                        _ =>  {}
                    }

                    if need_configure.load(atomic::Ordering::Acquire) && last_checked_time.elapsed() > idle_configure_timeout {
                        let last_xid = *last_configure_xid.lock().unwrap();
                        if ctx.is_window_concerned(last_xid) {
                            wm_debug!("timedout, reload");
                            println!("delayed configure 0x{:x} ", last_xid);

                            let diff = if filter.show_diff() {
                                Some(hashset!(last_xid))
                            } else {
                                None
                            };
                            //TODO: update the pinned list
                            
                            //FIXME: we do full collect_windows here because I have no facility
                            //to track stacking operations yet.
                            ctx.refresh_windows();
                            ctx.dump_windows(diff);
                            need_configure.store(false, atomic::Ordering::Release);
                        }
                    }
                }

            });
        }


        loop {
            if let Some(ev) = ctx.c.poll_for_event() {
                match ev.response_type() & !0x80 {
                    xcb::xproto::CREATE_NOTIFY => {
                        let cne = as_event::<xcb::CreateNotifyEvent>(&ev);
                        if cne.parent() != screen.root() {
                            break;
                        }
                        println!("create 0x{:x}, parent 0x{:x}", cne.window(), cne.parent());

                        // assumes that window will be at top when created
                        let new_win = ctx.query_window(cne.window());
                        ctx.update_with(new_win);
                        let diff = if filter.show_diff() {
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
                            if rne.parent() != screen.root() {
                                println!("reparent 0x{:x} to 0x{:x}", rne.window(), rne.parent());
                                ctx.remove(rne.window());

                                ctx.dump_windows(None);

                            } else {
                                println!("reparent 0x{:x} to root", rne.window());
                                let new_win = ctx.query_window(rne.window());
                                ctx.update_with(new_win);

                                let diff = if filter.show_diff() {
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

                        //TODO: take care other CNE cases
                        if ctx.is_window_concerned(cne.window()) && ctx.is_window_concerned(cne.above_sibling()) {
                            if *last_configure_xid.lock().unwrap() != cne.window() {
                                println!("configure 0x{:x} above: 0x{:x}", cne.window(), cne.above_sibling());
                                let diff = if filter.show_diff() {
                                    Some(hashset!(cne.window(), cne.above_sibling()))
                                } else {
                                    None
                                };

                                //FIXME: we do full collect_windows here because I have no facility
                                //to track stacking operations yet.
                                ctx.refresh_windows();
                                ctx.dump_windows(diff);
                                *last_configure_xid.lock().unwrap() = cne.window();
                                tx.send(Message::Reset).unwrap();

                            } else {
                                tx.send(Message::TimeoutEvent).unwrap();
                            }
                        }
                    },

                    xproto::MAP_NOTIFY => {
                        let mn = as_event::<xcb::MapNotifyEvent>(&ev);

                        if ctx.is_window_concerned(mn.window()) {
                            {
                                let mut windows = ctx.get_windows();
                                let win = windows.get_mut(&mn.window()).unwrap();
                                win.attrs.map_state = MapState::Viewable;

                                println!("map 0x{:x}", mn.window());

                                ctx.update_pin_state(win);
                            }

                            let diff = if filter.show_diff() {
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
                            {
                                let mut windows = ctx.get_windows();
                                let win = windows.get_mut(&un.window()).unwrap();
                                win.attrs.map_state = MapState::Unmapped;
                                ctx.update_pin_state(win);
                            }
                            println!("unmap 0x{:x}", un.window());
                            ctx.dump_windows(None);
                        }
                    },


                    _ => {
                    },
                } 
            };

            thread::sleep(time::Duration::from_millis(50));
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

