extern crate xcb;
extern crate getopts;
extern crate colored;

use colored::*;
use std::fmt::*;
use std::env;

#[derive(Debug, Clone, Copy)]
struct Geometry<T> where T: Copy {
    x: T,
    y: T,
    width: T,
    height: T,
}

#[derive(Debug, Copy, Clone)]
enum MapState {
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

#[derive(Debug)]
struct Attributes {
    override_redirect: bool,
    map_state: MapState,
}

#[derive(Debug)]
struct Window {
    id: xcb::ffi::xcb_window_t,
    name: String,
    attrs: Attributes,
    geom: Geometry<i32>,
    valid: bool,
}

impl Display for Window {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let geom_str = format!("{}x{}+{}+{}", self.geom.width, self.geom.height,
           self.geom.x, self.geom.y).red();
        let id = format!("0x{:x}", self.id).blue();
        let or = format!("{}", if self.attrs.override_redirect {
            "OR"
        } else {
            ""
        }).green();
        let state = format!("{}", match self.attrs.map_state {
            MapState::Unmapped => "Unmapped",
            MapState::Unviewable => "Unviewable",
            MapState::Viewable => "Viewable"
        }).cyan();
        write!(f, "{}({}) {} {} {}", id, self.name.cyan(), geom_str, or, state)
    }
}

enum XcbRequest<'a> {
    GWA(xcb::GetWindowAttributesCookie<'a>),
    GE(xcb::GetGeometryCookie<'a>),
    GP(xcb::GetPropertyCookie<'a>),
}

fn usage(program: &String, opts: &getopts::Options)
{
    let brief = format!("Usage: {} [vcmof]", program);
    println!("{}", opts.usage(&brief));
}

fn query_windows(c: &xcb::Connection, res: &xcb::QueryTreeReply) -> Vec<Window> {
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

pub fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let program = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optflag("v", "only-mapped", "show only mapped windows");
	opts.optflag("c", "colored", "output info with color");
	opts.optflag("m", "monitor", "run in monitor mode");
	//opts.optflag("f", "filter", "filter rule");
	opts.optflag("o", "omit-hidden", "omit hidden windows");
	opts.optflag("h", "help", "show this help");

    let args = match opts.parse(&args) {
        Ok(m) => m,
        Err(_) => { usage(&program, &opts); return; },
    };

    if args.opt_present("h") {
        usage(&program, &opts);
        return;
    }

    let (c, _) = xcb::Connection::connect(None).unwrap();
    let screen = c.get_setup().roots().next().unwrap();

    let res = match xcb::query_tree(&c, screen.root()).get_reply() {
        Ok(result) => result,
        Err(_) => return,
    };

    let windows = query_windows(&c, &res);


    let mut target_windows = windows;
    
    macro_rules! do_filter {
        ($windows:ident, $op:ident, $rule:expr) => (
            $windows = $windows.into_iter(). $op ( $rule ) .collect::<Vec<_>>();
        )
    }

    if args.opt_present("o") || args.opt_present("v") {
        let mut has_guard_window = false;
        for w in &target_windows {
            if w.name.contains("guard window") && w.attrs.override_redirect {
                has_guard_window = true;
                break;
            }
        };

        do_filter!(target_windows, skip_while, |ref w| {
            if has_guard_window {
                !w.name.contains("guard window") || !w.attrs.override_redirect
            } else {
                false
            }
        });

        if args.opt_present("v") {
            do_filter!(target_windows, filter, |w: &Window| { w.attrs.map_state == MapState::Viewable });
        }

        if args.opt_present("o") {
            do_filter!(target_windows, filter, |ref w| {
                w.geom.x < screen.width_in_pixels() as i32 && w.geom.y < screen.height_in_pixels() as i32 &&
                    w.geom.width + w.geom.x > 0 && w.geom.height + w.geom.y > 0
            });
        }
    }

    if args.opt_present("m") {
        monitor(&target_windows);
    } else {
        dump_windows(&target_windows, args.opt_present("c"));
    }
}


fn monitor(windows: &Vec<Window>) {
}
fn win2str(w: &Window, colored: bool) -> String {
    let geom_str = format!("{}x{}+{}+{}", w.geom.width, w.geom.height,
                           w.geom.x, w.geom.y).red();
    let id = format!("0x{:x}", w.id).blue();
    let or = format!("{}", if w.attrs.override_redirect {
        "OR"
    } else {
        ""
    }).green();
    let state = format!("{}", match w.attrs.map_state {
        MapState::Unmapped => "Unmapped",
        MapState::Unviewable => "Unviewable",
        MapState::Viewable => "Viewable"
    }).cyan();

    if colored {
        format!("{}({}) {} {} {}", id, w.name.cyan(), geom_str, or, state)
    } else {
        format!("{}({}) {} {} {}", id.normal(), w.name, geom_str.normal(), or.normal(), state.normal())
    }
}

fn dump_windows(windows: &Vec<Window>, colored: bool) {
    for (i, w) in windows.into_iter().enumerate() {
        println!("{}: {}", i, win2str(w, colored));
    }
}

