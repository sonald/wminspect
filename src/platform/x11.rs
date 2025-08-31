/// x11-specific functionality
#[cfg(feature = "x11")]
use colored::*;
#[cfg(feature = "x11")]
use xcb::xproto;
#[cfg(feature = "x11")]
use xcb_util::ewmh;

#[cfg(feature = "x11")]
use crate::core::state::*;
#[cfg(feature = "x11")]
use crate::core::types::{
    Attributes, Condition, Geometry, MapState, Window as CoreWindow, WindowListView,
};
#[cfg(feature = "x11")]
use crate::dsl::Filter;
#[cfg(feature = "x11")]
use crate::{wm_info, wm_trace};

// XCB event handling functions removed due to API changes

#[cfg(feature = "x11")]
fn get_tty_cols() -> Option<usize> {
    unsafe {
        let mut winsz = std::mem::MaybeUninit::<libc::winsize>::uninit();
        match libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, winsz.as_mut_ptr()) {
            0 => Some(winsz.assume_init().ws_col as usize),
            _ => None,
        }
    }
}

#[cfg(feature = "x11")]
fn win2str(w: &CoreWindow, mut colored: bool) -> String {
    let geom_str = format!("{}", w.geom);
    let id = format!("0x{:x}", w.id);
    let attrs = format!("{}", w.attrs);

    if unsafe { libc::isatty(libc::STDOUT_FILENO) } == 0 {
        colored = false;
    }
    let cols = get_tty_cols().unwrap_or(80) / 2;
    let name = w.name.chars().take(cols).collect::<String>();

    if colored {
        format!(
            "{}({}) {} {}",
            id.blue(),
            name.cyan(),
            geom_str.red(),
            attrs.green()
        )
    } else {
        format!("{}({}) {} {}", id, w.name, geom_str, attrs)
    }
}

#[cfg(feature = "x11")]
pub struct Context<'a> {
    pub c: &'a ewmh::Connection,
    pub root: u32,
    state: StateRef,
}

#[cfg(feature = "x11")]
impl<'a> Context<'a> {
    pub fn new(c: &'a ewmh::Connection, f: Filter) -> Context<'a> {
        let screen = c.get_setup().roots().next().unwrap();
        let state = create_state_ref(f);

        Context {
            c,
            root: screen.root(),
            state,
        }
    }

    pub fn set_mapped_only(&mut self) {
        self.state.add_option(Condition::MappedOnly);
    }

    pub fn set_colorful(&mut self) {
        self.state.add_option(Condition::Colorful);
    }

    pub fn set_omit_hidden(&mut self) {
        self.state.add_option(Condition::OmitHidden);
    }

    pub fn set_no_special(&mut self) {
        self.state.add_option(Condition::NoSpecial);
    }

    pub fn set_show_diff(&mut self) {
        self.state.add_option(Condition::ShowDiff);
    }

    pub fn set_clients_only(&mut self) {
        self.state.add_option(Condition::ClientsOnly);
    }

    pub fn refresh_windows(&self) {
        wm_info!("Refreshing windows...");

        let tree_cookie = xproto::query_tree(self.c, self.root);
        if let Ok(tree) = tree_cookie.get_reply() {
            let children = tree.children();
            let mut layout = self.state.write_layout();
            layout.clear();

            for win in children {
                let attrs_cookie = xproto::get_window_attributes(self.c, *win);
                let geom_cookie = xproto::get_geometry(self.c, *win);
                let name_cookie = ewmh::get_wm_name(self.c, *win);

                let attrs = attrs_cookie.get_reply().ok();
                let geom = geom_cookie.get_reply().ok();
                let name = name_cookie
                    .get_reply()
                    .ok()
                    .map(|r| r.string().to_string())
                    .unwrap_or_default();

                let attributes = attrs
                    .map(|a| Attributes {
                        override_redirect: a.override_redirect(),
                        map_state: match a.map_state() as u32 {
                            xproto::MAP_STATE_UNMAPPED => MapState::Unmapped,
                            xproto::MAP_STATE_UNVIEWABLE => MapState::Unviewable,
                            _ => MapState::Viewable,
                        },
                    })
                    .unwrap_or(Attributes {
                        override_redirect: false,
                        map_state: MapState::Unmapped,
                    });

                let geometry = geom
                    .map(|g| Geometry {
                        x: g.x(),
                        y: g.y(),
                        width: g.width(),
                        height: g.height(),
                    })
                    .unwrap_or(Geometry {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    });

                let window = CoreWindow {
                    id: *win,
                    name,
                    attrs: attributes,
                    geom: geometry,
                    valid: true,
                };

                let include = self.state.read_filter().apply_to(&window);
                layout.stack_view.push(*win);
                if include {
                    layout.filtered_view.push(*win);
                }
                layout.insert_window(window);
            }
        }
    }

    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let layout = self.state.read_layout();
        let colored = self.state.has_option(&Condition::Colorful);
        let show_diff = self.state.has_option(&Condition::ShowDiff);

        for (i, wid) in layout.filtered_view.iter().enumerate() {
            if let Some(w) = layout.windows.get(wid) {
                if show_diff && changes.is_some() && changes.as_ref().unwrap().contains(&wid) {
                    println!("{}: {}", i, win2str(w, colored).on_white());
                } else {
                    println!("{}: {}", i, win2str(w, colored));
                }
            }
        }
    }
}

#[cfg(feature = "x11")]
pub fn monitor(ctx: &Context) {
    wm_info!("Starting monitor mode...");

    ctx.refresh_windows();
    ctx.dump_windows(None);

    while let Some(_event) = ctx.c.poll_for_event() {
        wm_trace!("event received");
        ctx.refresh_windows();
        ctx.dump_windows(None);
    }

    wm_info!("Monitor mode started - basic implementation");
}

#[cfg(not(feature = "x11"))]
pub fn x11_specific_functionality() {
    // placeholder for non-X11 builds
}
