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

    /// Check if a window should be included based on command-line conditions
    fn check_window_conditions(&self, window: &CoreWindow, attrs: &Attributes) -> bool {
        // Check MappedOnly condition - only show mapped windows
        if self.state.has_option(&Condition::MappedOnly) && 
           attrs.map_state != MapState::Viewable {
            return false;
        }
        
        // Check OmitHidden condition - exclude hidden/iconified windows  
        if self.state.has_option(&Condition::OmitHidden) && 
           attrs.map_state == MapState::Unviewable {
            return false;
        }
        
        // Check NoSpecial condition - ignore special windows (docks, panels, etc.)
        if self.state.has_option(&Condition::NoSpecial) && 
           self.is_special_window(window, attrs) {
            return false;
        }
        
        // Check ClientsOnly condition - only include client-managed windows
        if self.state.has_option(&Condition::ClientsOnly) && 
           attrs.override_redirect {
            return false;
        }
        
        true
    }

    /// Detect special windows (docks, panels, tooltips, etc.)
    fn is_special_window(&self, window: &CoreWindow, attrs: &Attributes) -> bool {
        // Override-redirect windows are typically special (tooltips, popups, etc.)
        if attrs.override_redirect {
            return true;
        }
        
        // Very small windows are often special (1x1 tracking windows, etc.)
        if window.geom.width <= 1 || window.geom.height <= 1 {
            return true;
        }
        
        // Windows with certain name patterns are often special
        let name_lower = window.name.to_lowercase();
        if name_lower.contains("dock") || 
           name_lower.contains("panel") ||
           name_lower.contains("toolbar") ||
           name_lower.contains("desktop") ||
           name_lower.contains("notification") ||
           name_lower.starts_with("xfce4-") ||
           name_lower.starts_with("gnome-") {
            return true;
        }
        
        false
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

                // First check command-line conditions before applying user-defined filters
                if self.check_window_conditions(&window, &attributes) {
                    let include = self.state.read_filter().apply_to(&window);
                    layout.stack_view.push(*win);
                    if include {
                        layout.filtered_view.push(*win);
                    }
                    layout.insert_window(window);
                }
            }
        }
    }

    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let layout = self.state.read_layout();
        let colored = self.state.has_option(&Condition::Colorful);
        let show_diff = self.state.has_option(&Condition::ShowDiff);

        for (i, wid) in layout.filtered_view.iter().enumerate() {
            if let Some(w) = layout.windows.get(wid) {
                if show_diff && changes.is_some() && changes.as_ref().unwrap().contains(wid) {
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

#[cfg(all(test, feature = "x11"))]
mod tests {
    use super::*;
    use crate::core::types::{Attributes, MapState, Geometry};
    use crate::dsl::Filter;

    // Helper struct to test window conditions without requiring real XCB connection
    struct TestContext {
        state: StateRef,
    }

    impl TestContext {
        fn new() -> Self {
            let filter = Filter::new();
            let state = crate::core::state::create_state_ref(filter);
            Self { state }
        }

        fn add_condition(&self, condition: Condition) {
            self.state.add_option(condition);
        }

        fn has_option(&self, condition: &Condition) -> bool {
            self.state.has_option(condition)
        }

        // Test version of check_window_conditions
        fn check_window_conditions(&self, window: &CoreWindow, attrs: &Attributes) -> bool {
            // Check MappedOnly condition - only show mapped windows
            if self.has_option(&Condition::MappedOnly) && 
               attrs.map_state != MapState::Viewable {
                return false;
            }
            
            // Check OmitHidden condition - exclude hidden/iconified windows  
            if self.has_option(&Condition::OmitHidden) && 
               attrs.map_state == MapState::Unviewable {
                return false;
            }
            
            // Check NoSpecial condition - ignore special windows (docks, panels, etc.)
            if self.has_option(&Condition::NoSpecial) && 
               self.is_special_window(window, attrs) {
                return false;
            }
            
            // Check ClientsOnly condition - only include client-managed windows
            if self.has_option(&Condition::ClientsOnly) && 
               attrs.override_redirect {
                return false;
            }
            
            true
        }

        // Test version of is_special_window
        fn is_special_window(&self, window: &CoreWindow, attrs: &Attributes) -> bool {
            // Override-redirect windows are typically special (tooltips, popups, etc.)
            if attrs.override_redirect {
                return true;
            }
            
            // Very small windows are often special (1x1 tracking windows, etc.)
            if window.geom.width <= 1 || window.geom.height <= 1 {
                return true;
            }
            
            // Windows with certain name patterns are often special
            let name_lower = window.name.to_lowercase();
            if name_lower.contains("dock") || 
               name_lower.contains("panel") ||
               name_lower.contains("toolbar") ||
               name_lower.contains("desktop") ||
               name_lower.contains("notification") ||
               name_lower.starts_with("xfce4-") ||
               name_lower.starts_with("gnome-") {
                return true;
            }
            
            false
        }
    }

    fn create_test_window(name: &str, map_state: MapState, override_redirect: bool, width: u16, height: u16) -> (CoreWindow, Attributes) {
        let window = CoreWindow {
            id: 1,
            name: name.to_string(),
            attrs: Attributes {
                map_state,
                override_redirect,
            },
            geom: Geometry {
                x: 0,
                y: 0,
                width,
                height,
            },
            valid: true,
        };
        
        let attrs = Attributes {
            map_state,
            override_redirect,
        };
        
        (window, attrs)
    }

    #[test]
    fn test_mapped_only_condition() {
        let ctx = TestContext::new();
        
        // Add MappedOnly condition
        ctx.add_condition(Condition::MappedOnly);

        // Test mapped window - should be included
        let (mapped_window, mapped_attrs) = create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&mapped_window, &mapped_attrs));

        // Test unmapped window - should be excluded
        let (unmapped_window, unmapped_attrs) = create_test_window("test", MapState::Unmapped, false, 100, 100);
        assert!(!ctx.check_window_conditions(&unmapped_window, &unmapped_attrs));
    }

    #[test]
    fn test_omit_hidden_condition() {
        let ctx = TestContext::new();
        
        // Add OmitHidden condition
        ctx.add_condition(Condition::OmitHidden);

        // Test viewable window - should be included
        let (viewable_window, viewable_attrs) = create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&viewable_window, &viewable_attrs));

        // Test unviewable (hidden) window - should be excluded
        let (hidden_window, hidden_attrs) = create_test_window("test", MapState::Unviewable, false, 100, 100);
        assert!(!ctx.check_window_conditions(&hidden_window, &hidden_attrs));
    }

    #[test]
    fn test_clients_only_condition() {
        let ctx = TestContext::new();
        
        // Add ClientsOnly condition
        ctx.add_condition(Condition::ClientsOnly);

        // Test client window (not override-redirect) - should be included
        let (client_window, client_attrs) = create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&client_window, &client_attrs));

        // Test override-redirect window - should be excluded
        let (popup_window, popup_attrs) = create_test_window("test", MapState::Viewable, true, 100, 100);
        assert!(!ctx.check_window_conditions(&popup_window, &popup_attrs));
    }

    #[test]
    fn test_no_special_condition() {
        let ctx = TestContext::new();
        
        // Add NoSpecial condition
        ctx.add_condition(Condition::NoSpecial);

        // Test normal window - should be included
        let (normal_window, normal_attrs) = create_test_window("firefox", MapState::Viewable, false, 800, 600);
        assert!(ctx.check_window_conditions(&normal_window, &normal_attrs));

        // Test dock window - should be excluded
        let (dock_window, dock_attrs) = create_test_window("xfce4-panel", MapState::Viewable, false, 800, 30);
        assert!(!ctx.check_window_conditions(&dock_window, &dock_attrs));

        // Test override-redirect window - should be excluded
        let (or_window, or_attrs) = create_test_window("tooltip", MapState::Viewable, true, 100, 50);
        assert!(!ctx.check_window_conditions(&or_window, &or_attrs));

        // Test tiny window - should be excluded
        let (tiny_window, tiny_attrs) = create_test_window("tracking", MapState::Viewable, false, 1, 1);
        assert!(!ctx.check_window_conditions(&tiny_window, &tiny_attrs));
    }

    #[test]
    fn test_special_window_detection() {
        let ctx = TestContext::new();

        // Test various special window patterns
        let test_cases = vec![
            ("xfce4-panel", MapState::Viewable, false, 800, 30, true),    // XFCE panel
            ("gnome-shell", MapState::Viewable, false, 1920, 1080, true), // GNOME shell
            ("dock", MapState::Viewable, false, 60, 800, true),           // Dock
            ("Desktop", MapState::Viewable, false, 1920, 1080, true),     // Desktop
            ("notification", MapState::Viewable, false, 300, 100, true),  // Notification
            ("tooltip", MapState::Viewable, true, 100, 50, true),         // Override-redirect
            ("tracking", MapState::Viewable, false, 1, 1, true),          // Tiny window
            ("firefox", MapState::Viewable, false, 800, 600, false),      // Normal window
            ("text-editor", MapState::Viewable, false, 600, 400, false),  // Normal window
        ];

        for (name, map_state, or, width, height, should_be_special) in test_cases {
            let (window, attrs) = create_test_window(name, map_state, or, width, height);
            let is_special = ctx.is_special_window(&window, &attrs);
            assert_eq!(is_special, should_be_special, 
                "Window '{}' special detection failed: expected {}, got {}", 
                name, should_be_special, is_special);
        }
    }

    #[test]
    fn test_multiple_conditions() {
        let ctx = TestContext::new();
        
        // Add multiple conditions
        ctx.add_condition(Condition::MappedOnly);
        ctx.add_condition(Condition::ClientsOnly);

        // Test window that meets both conditions - should be included
        let (good_window, good_attrs) = create_test_window("firefox", MapState::Viewable, false, 800, 600);
        assert!(ctx.check_window_conditions(&good_window, &good_attrs));

        // Test unmapped client window - should be excluded (fails MappedOnly)
        let (unmapped_window, unmapped_attrs) = create_test_window("firefox", MapState::Unmapped, false, 800, 600);
        assert!(!ctx.check_window_conditions(&unmapped_window, &unmapped_attrs));

        // Test mapped override-redirect window - should be excluded (fails ClientsOnly)
        let (popup_window, popup_attrs) = create_test_window("popup", MapState::Viewable, true, 200, 100);
        assert!(!ctx.check_window_conditions(&popup_window, &popup_attrs));
    }
}
