extern crate xcb;
extern crate getopts;
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
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use super::filter::*;

macro_rules! wm_debug {
    ( $($a:tt)* ) => (
        if cfg!(debug_assertions) {
            println!{$($a)*}; 
        })
}

#[cfg(feature = "core_intrinsics")]
fn print_type_of<T>(_: &T) {
    wm_debug!("{}", unsafe { std::intrinsics::type_name::<T>() });
}

#[cfg(not(feature = "core_intrinsics"))]
fn print_type_of<T>(_: &T) {
}

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

#[derive(Debug, Copy, Clone)]
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

impl Eq for MapState { }

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

impl Display for Window {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let id = format!("0x{:x}", self.id);
        write!(f, "{}({}) {} {}", id, self.name, self.geom, self.attrs)
    }
}

pub enum XcbRequest<'a> {
    GWA(xcb::GetWindowAttributesCookie<'a>),
    GE(xcb::GetGeometryCookie<'a>),
    GP(xcb::GetPropertyCookie<'a>),
}

pub fn query_window(c: &xcb::Connection, id: xcb::Window) -> Window {
    let net_wm_name_atom = xcb::intern_atom(&c, false, "_NET_WM_NAME").get_reply().unwrap();
    let utf8_string_atom = xcb::intern_atom(&c, false, "UTF8_STRING").get_reply().unwrap();

    let mut qs: Vec<XcbRequest> = Vec::new();
    qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, id)));
    qs.push(XcbRequest::GE(xcb::get_geometry(&c, id)));
    qs.push(XcbRequest::GP(xcb::get_property(&c, false, id, net_wm_name_atom.atom(),
        utf8_string_atom.atom(), 0, std::u32::MAX)));
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

pub fn query_windows(c: &xcb::Connection, res: &xcb::QueryTreeReply) -> Vec<Window> {
    let net_wm_name_atom = xcb::intern_atom(&c, false, "_NET_WM_NAME").get_reply().unwrap();
    let utf8_string_atom = xcb::intern_atom(&c, false, "UTF8_STRING").get_reply().unwrap();

    let mut qs: Vec<XcbRequest> = Vec::new();
    for w in res.children() {
        qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, *w)));
        qs.push(XcbRequest::GE(xcb::get_geometry(&c, *w)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, net_wm_name_atom.atom(),
                                                  utf8_string_atom.atom(), 0, std::u32::MAX)));
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

macro_rules! do_filter {
    ($windows:ident, $op:ident, $rule:expr) => (
        $windows = $windows.into_iter(). $op ( $rule ) .collect::<Vec<_>>();
        )
}

fn is_window_pinned(w: &Window, filter: &Filter) -> bool {
    for rule in &filter.rules {
        if rule.action == Action::Pin && rule.func.as_ref()(w) {
            return true;
        }
    }

    false
}

fn collect_pinned_windows(windows: &Vec<Window>, filter: &Filter) -> HashSet<xcb::Window> {
    let f = |w: &Window| {
        for rule in &filter.rules {
            if rule.action == Action::Pin && rule.func.as_ref()(w) {
                return Some(w.id.clone());
            }
        }
        None
    };

    windows.iter().filter_map(f).collect()
}

pub fn collect_windows(c: &xcb::Connection, filter: &Filter) -> Vec<Window> {
    let screen = c.get_setup().roots().next().unwrap();
    let res = match xcb::query_tree(&c, screen.root()).get_reply() {
        Ok(result) => result,
        Err(_) => return Vec::new(),
    };

    let mut target_windows = query_windows(&c, &res);

    if filter.mapped_only() || filter.omit_hidden() {
        let has_guard_window = target_windows.iter()
            .any(|ref w| w.name.contains("guard window") && w.attrs.override_redirect);

        do_filter!(target_windows, skip_while, |ref w| {
            if has_guard_window {
                !w.name.contains("guard window") || !w.attrs.override_redirect
            } else {
                false
            }
        });

        if filter.mapped_only() {
            do_filter!(target_windows, filter, |w: &Window| { w.attrs.map_state == MapState::Viewable });
        }

        if filter.omit_hidden() {
            do_filter!(target_windows, filter, |ref w| {
                w.geom.x < screen.width_in_pixels() as i32 && w.geom.y < screen.height_in_pixels() as i32 &&
                    w.geom.width + w.geom.x > 0 && w.geom.height + w.geom.y > 0
            });
        }
    }

    if filter.no_special() {
        let mut specials = HashSet::new();
        specials.insert("mutter guard window");
        specials.insert("deepin-metacity guard window");
        specials.insert("mutter topleft corner window");
        specials.insert("deepin-metacity topleft corner window");
        do_filter!(target_windows, filter, |w: &Window| { !specials.contains(w.name.as_str()) });
    }

    if filter.rules.len() > 0 {
        for rule in &filter.rules {
            if rule.action == Action::FilterOut {
                do_filter!(target_windows, filter, rule.func.as_ref());
            }
        }
    }

    return target_windows;
}

pub fn dump_windows(windows: &Vec<Window>, filter: &Filter, changes: &HashSet<xcb::Window>) {
    let colored = filter.colorful();
    for (i, w) in windows.iter().enumerate() {
        if filter.show_diff() && changes.contains(&w.id) {
            println!("{}: {}", i, win2str(w, colored).on_white());
        } else {
            println!("{}: {}", i, win2str(w, colored));
        }
    }
}


#[derive(Debug, Clone)]
pub enum Message {
    TimeoutEvent,
    Reset,
    Quit,
}

macro_rules! hashset {
    ( $( $e:expr )* ) => ({
        let mut h = HashSet::new();
        $( h.insert($e); )*
        h
    })
}

pub fn monitor(c: &xcb::Connection, screen: &xcb::Screen, filter: &Filter) {
    let c = Arc::new(c);

    let ev_mask: u32 = xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY;
    xcb::xproto::change_window_attributes(&c, screen.root(), 
                                          &[(xcb::xproto::CW_EVENT_MASK, ev_mask)]);
    c.flush();

    let windows = Arc::new(Mutex::new(collect_windows(&c, filter)));
    let last_configure_xid = Arc::new(Mutex::new(xcb::WINDOW_NONE));
    let need_configure = AtomicBool::new(false);
    let diff = Arc::new(Mutex::new(collect_pinned_windows(windows.lock().unwrap().as_ref(), filter)));
    let (tx, rx) = mpsc::channel::<Message>();

    dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
    print_type_of(&windows);

    crossbeam::scope(|scope| {

        {
            let windows = windows.clone();
            let diff = diff.clone();
            let c = c.clone();
            let last_configure_xid = last_configure_xid.clone();

            scope.spawn(move || {
                let event_related = |ev_win: xcb::Window| windows.lock().unwrap().iter().any(|ref w| w.id == ev_win);
                print_type_of(filter);

                let idle_configure_timeout = time::Duration::from_millis(200);
                let mut last_checked_time = time::Instant::now();

                loop {
                    match rx.recv_timeout(time::Duration::from_millis(10)) {
                        Ok(Message::TimeoutEvent) => { 
                            wm_debug!("recv timeout"); 
                            last_checked_time = time::Instant::now();
                            need_configure.store(true, Ordering::Release)
                        },
                        Ok(Message::Reset) => { 
                            need_configure.store(false, Ordering::Release)
                        },
                        Ok(Message::Quit) => { break; },
                        _ =>  {}
                    }

                    if need_configure.load(Ordering::Acquire) && last_checked_time.elapsed() > idle_configure_timeout {
                        if event_related(*last_configure_xid.lock().unwrap()) {
                        }

                        wm_debug!("timedout, reload");
                        println!("delayed configure 0x{:x} ", *last_configure_xid.lock().unwrap());

                        *windows.lock().unwrap() = collect_windows(&c, filter);
                        //configure should never change diff set in my knowledge, so leave diff as
                        //it was.
                        assert!(diff.lock().unwrap().contains(&last_configure_xid.lock().unwrap()));

                        dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
                        need_configure.store(false, Ordering::Release);
                    }
                }

            });
        }


        let event_related = |ev_win: xcb::Window| windows.lock().unwrap().iter().any(|ref w| w.id == ev_win);
        loop {
            if let Some(ev) = c.poll_for_event() {
                match ev.response_type() & !0x80 {
                    xcb::xproto::CREATE_NOTIFY => {
                        let cne = xcb::cast_event::<xcb::CreateNotifyEvent>(&ev);
                        if cne.parent() != screen.root() {
                            break;
                        }
                        println!("create 0x{:x}, parent 0x{:x}", cne.window(), cne.parent());

                        // assumes that window will be at top when created
                        let new_win = query_window(&c, cne.window());
                        {
                            let mut locked = diff.lock().unwrap();
                            if is_window_pinned(&new_win, filter) {
                                locked.insert(cne.window());
                            }
                        }
                        windows.lock().unwrap().push(new_win);
                        dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
                    },
                    xcb::xproto::DESTROY_NOTIFY => {
                        let dne = xcb::cast_event::<xcb::DestroyNotifyEvent>(&ev);

                        if event_related(dne.window()) {
                            println!("destroy 0x{:x}", dne.window());
                            windows.lock().unwrap().retain(|ref w| w.id != dne.window());
                            {
                                let mut locked = diff.lock().unwrap();
                                if locked.contains(&dne.window()) {
                                    locked.remove(&dne.window());
                                }
                            }
                            dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
                        }
                    },

                    xcb::xproto::REPARENT_NOTIFY => {
                        let rne = xcb::cast_event::<xcb::ReparentNotifyEvent>(&ev);

                        if event_related(rne.window()) {
                            let mut windows = windows.lock().unwrap();
                            if rne.parent() != screen.root() {
                                println!("reparent 0x{:x} to 0x{:x}", rne.window(), rne.parent());
                                windows.retain(|ref w| w.id != rne.window());
                                {
                                    let mut locked = diff.lock().unwrap();
                                    if locked.contains(&rne.window()) {
                                        locked.remove(&rne.window());
                                    }
                                }
                                dump_windows(&windows, filter, &diff.lock().unwrap());

                            } else {
                                println!("reparent 0x{:x} to root", rne.window());
                                let new_win = query_window(&c, rne.window());
                                {
                                    let mut locked = diff.lock().unwrap();
                                    if is_window_pinned(&new_win, filter) {
                                        locked.insert(rne.window());
                                    }
                                }
                                windows.push(new_win);
                                dump_windows(&windows, filter, &diff.lock().unwrap());
                            }
                        }
                    },

                    xproto::CONFIGURE_NOTIFY => {
                        let cne = xcb::cast_event::<xcb::ConfigureNotifyEvent>(&ev);

                        if event_related(cne.window()) {
                            if *last_configure_xid.lock().unwrap() != cne.window() {
                                println!("configure 0x{:x} above: 0x{:x}", cne.window(), cne.above_sibling());
                                *windows.lock().unwrap() = collect_windows(&c, filter);
                                dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
                                *last_configure_xid.lock().unwrap() = cne.window();
                                tx.send(Message::Reset).unwrap();

                            } else {
                                tx.send(Message::TimeoutEvent).unwrap();
                            }
                        }
                    },

                    xproto::MAP_NOTIFY => {
                        let mn = xcb::cast_event::<xcb::MapNotifyEvent>(&ev);

                        if event_related(mn.window()) {
                            let mut windows = windows.lock().unwrap();
                            {
                                let mut win = windows.iter_mut().find(|ref mut w| w.id == mn.window()).unwrap();
                                win.attrs.map_state = MapState::Viewable;
                                println!("map 0x{:x}", mn.window());

                                let mut locked = diff.lock().unwrap();
                                if is_window_pinned(&win, filter) {
                                    locked.insert(mn.window());
                                }
                            }
                            dump_windows(&windows, filter, &diff.lock().unwrap());
                        }
                    },

                    xproto::UNMAP_NOTIFY => {
                        let un = xcb::cast_event::<xcb::UnmapNotifyEvent>(&ev);

                        if event_related(un.window()) {
                            {
                                let mut locked = windows.lock().unwrap();
                                let mut win = locked.iter_mut().find(|ref w| w.id == un.window()).unwrap();
                                win.attrs.map_state = MapState::Unmapped;
                            }
                            println!("unmap 0x{:x}", un.window());
                            {
                                let mut locked = diff.lock().unwrap();
                                if locked.contains(&un.window()) {
                                    locked.remove(&un.window());
                                }
                            }
                            dump_windows(&windows.lock().unwrap(), filter, &diff.lock().unwrap());
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

