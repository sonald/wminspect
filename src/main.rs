extern crate xcb;
extern crate getopts;

use std::io::Write;
use std::fmt::*;

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
        write!(f, "0x{:x}({}) {}x{}+{}+{}", self.id, self.name,
            self.geom.width, self.geom.height, self.geom.x, self.geom.y)
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

    let net_wm_name_atom = xcb::intern_atom(&c, false, "_NET_WM_NAME").get_reply().unwrap();
    let utf8_string_atom = xcb::intern_atom(&c, false, "UTF8_STRING").get_reply().unwrap();

    let mut qs: Vec<XcbRequest> = Vec::new();
    for w in res.children() {
        qs.push(XcbRequest::GWA(xcb::get_window_attributes(&c, *w)));
        qs.push(XcbRequest::GE(xcb::get_geometry(&c, *w)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, net_wm_name_atom.atom(),
                                                  utf8_string_atom.atom(), 0, std::u32::MAX)));
        qs.push(XcbRequest::GP(xcb::get_property(&c, false, *w, xcb::xproto::ATOM_WM_NAME, 
                                                  utf8_string_atom.atom(), 0, std::u32::MAX)));
    }

    let mut windows = Vec::with_capacity(res.children_len() as usize);

    macro_rules! apply_reply {
        ($win:ident $cookie:ident $reply:ident $e:expr) => (
            match $cookie.get_reply() {
                Ok($reply) => $e,
                Err(_) => $win.valid = false,
            })
    }

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


    let target_windows = windows.into_iter().filter(|ref w| w.valid && w.attrs.map_state == MapState::Viewable).collect::<Vec<_>>();

    for (i, w) in target_windows.into_iter().enumerate() {
        println!("{}: {}", i, w);
    }
}


