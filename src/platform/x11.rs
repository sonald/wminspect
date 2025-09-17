/// x11-specific functionality
#[cfg(feature = "x11")]
use xcb_util::ewmh;
#[cfg(feature = "x11")]
use colored::*;
#[cfg(feature = "x11")]
use xcb::xproto;
#[cfg(feature = "x11")]
use xcb;

#[cfg(feature = "x11")]
use crate::core::types::{Window as CoreWindow, WindowListView, Condition};
#[cfg(feature = "x11")]
use crate::core::state::*;
#[cfg(feature = "x11")]
use crate::dsl::Filter;
#[cfg(feature = "x11")]
use crate::{wm_info};


// XCB event handling functions removed due to API changes

#[cfg(feature = "x11")]
fn get_tty_cols() -> Option<usize> {
    unsafe {
        let mut winsz = std::mem::MaybeUninit::<libc::winsize>::uninit();
        match libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, winsz.as_mut_ptr()) {
            0 => {
                Some(winsz.assume_init().ws_col as usize)
            },
            _ => None
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
        format!("{}({}) {} {}", id.blue(), name.cyan(), geom_str.red(), attrs.green())
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
    
    pub fn set_no_override_redirect(&mut self) {
        self.state.add_option(Condition::NoOverrideRedirect);
    }
    
    pub fn refresh_windows(&self) {
        wm_info!("Refreshing windows...");
        
        let windows = self.collect_windows();
        
        // Update window data structures in a separate scope
        {
            let mut layout = self.state.write_layout();
            layout.clear();
            layout.stack_view = windows.iter().map(|w| w.id).collect();
            
            // Populate windows HashMap
            for window in windows {
                layout.insert_window(window);
            }
        } // layout write lock released here
        
        // Apply filtering rules
        wm_info!("Starting apply_filter...");
        self.apply_filter();
        
        // Re-read layout to get updated filtered_view
        let layout = self.state.read_layout();
        wm_info!("Refreshed {} windows, {} passed filter", layout.window_count(), layout.filtered_view.len());
    }
    
    /// Get client window list from window manager (_NET_CLIENT_LIST)
    fn get_client_windows(&self) -> Vec<u32> {
        let mut client_ids = Vec::new();
        
        // Try to get client list from EWMH
        let client_list_cookie = xcb_util::ewmh::get_client_list(&self.c, 0);
        if let Ok(reply) = client_list_cookie.get_reply() {
            client_ids = reply.windows().to_vec();
        }
        
        client_ids
    }

    fn collect_windows(&self) -> Vec<CoreWindow> {
        let mut windows = Vec::new();
        
        // Query the window tree from root
        let query_tree_cookie = xcb::query_tree(self.c, self.root);
        let query_tree_reply = match query_tree_cookie.get_reply() {
            Ok(reply) => reply,
            Err(e) => {
                wm_info!("Failed to query window tree: {:?}", e);
                return windows;
            }
        };
        
        let child_windows = query_tree_reply.children();
        wm_info!("Found {} child windows", child_windows.len());
        
        // Collect window information for each child
        for (i, &window_id) in child_windows.iter().enumerate() {
            if i % 20 == 0 {
                wm_info!("Processing window {}/{}", i + 1, child_windows.len());
            }
            match self.query_window_info(window_id) {
                Some(window) => {
                    windows.push(window);
                },
                None => {
                    // Skip windows we can't query (may be destroyed)
                    continue;
                }
            }
        }
        
        wm_info!("Successfully processed {} windows", windows.len());
        
        // Debug: Print first few windows
        for (i, window) in windows.iter().take(3).enumerate() {
            wm_info!("Window {}: 0x{:x} '{}' {}x{}", i, window.id, window.name, window.geom.width, window.geom.height);
        }
        
        windows
    }
    
    fn query_window_info(&self, window_id: u32) -> Option<CoreWindow> {
        // Get window attributes
        let attrs_cookie = xcb::get_window_attributes(self.c, window_id);
        let attrs_reply = attrs_cookie.get_reply().ok()?;
        
        // Get window geometry
        let geom_cookie = xcb::get_geometry(self.c, window_id);
        let geom_reply = geom_cookie.get_reply().ok()?;
        
        // Get window name using EWMH first, fall back to WM_NAME
        let name = self.get_window_name(window_id)
            .unwrap_or_else(|| format!("Window-{:x}", window_id));
        
        // Convert XCB types to our types
        let map_state_val = attrs_reply.map_state();
        let map_state = if map_state_val == xproto::MAP_STATE_UNMAPPED as u8 {
            crate::core::types::MapState::Unmapped
        } else if map_state_val == xproto::MAP_STATE_UNVIEWABLE as u8 {
            crate::core::types::MapState::Unviewable
        } else if map_state_val == xproto::MAP_STATE_VIEWABLE as u8 {
            crate::core::types::MapState::Viewable
        } else {
            crate::core::types::MapState::Unmapped
        };
        
        let attributes = crate::core::types::Attributes {
            override_redirect: attrs_reply.override_redirect(),
            map_state,
        };
        
        let geometry = crate::core::types::Geometry {
            x: geom_reply.x(),
            y: geom_reply.y(),
            width: geom_reply.width(),
            height: geom_reply.height(),
        };
        
        Some(CoreWindow {
            id: window_id,
            name,
            attrs: attributes,
            geom: geometry,
            valid: true,
        })
    }
    
    fn get_window_name(&self, window_id: u32) -> Option<String> {
        // Try EWMH _NET_WM_NAME first
        if let Some(name) = self.get_ewmh_window_name(window_id) {
            return Some(name);
        }
        
        // Fall back to WM_NAME
        self.get_wm_name(window_id)
    }
    
    fn get_ewmh_window_name(&self, window_id: u32) -> Option<String> {
        // Try to get the window name using EWMH _NET_WM_NAME
        let name_cookie = xcb_util::ewmh::get_wm_name(&self.c, window_id);
        if let Ok(name_reply) = name_cookie.get_reply() {
            return Some(name_reply.string().to_string());
        }
        None
    }
    
    fn get_wm_name(&self, window_id: u32) -> Option<String> {
        let wm_name_cookie = xcb::get_property(
            self.c,
            false,
            window_id,
            xproto::ATOM_WM_NAME,
            xproto::ATOM_STRING,
            0,
            1024
        );
        
        if let Ok(reply) = wm_name_cookie.get_reply() {
            if reply.value_len() > 0 {
                let name_bytes = reply.value();
                return String::from_utf8(name_bytes.to_vec()).ok();
            }
        }
        None
    }
    
    fn apply_filter(&self) {
        // Apply filtering rules to update filtered_view
        let filter = self.state.read_filter();
        
        wm_info!("Getting layout for filtering...");
        
        // Apply filtering based on command line options and DSL rules
        let filtered_view = {
            let layout = self.state.read_layout();
            let mut filtered_view = Vec::new();
            
            wm_info!("Filtering {} windows from stack_view", layout.stack_view.len());
            
            let options = self.state.read_options();
            let is_clients_only = options.iter().any(|opt| matches!(opt, crate::core::types::Condition::ClientsOnly));
            
            // Determine which windows to process based on options
            let window_ids_to_check: Vec<u32> = if is_clients_only {
                // For clients-only mode, only check EWMH client windows
                self.get_client_windows()
            } else {
                // For normal mode, check all windows from the stack_view (default behavior)
                layout.stack_view.clone()
            };
            
            for window_id in &window_ids_to_check {
                // Get window info (may need to query directly for client windows)
                let window = if is_clients_only && !layout.windows.contains_key(window_id) {
                    self.query_window_info(*window_id)
                } else {
                    layout.get_window(*window_id).cloned()
                };
                
                if let Some(window) = window {
                    // Apply command line options filtering
                    if !self.should_include_window(&window, &*options) {
                        continue;
                    }
                    
                    // Apply DSL filter rules (if any)
                    if filter.rule_count() > 0 && !filter.apply_to(&window) {
                        continue;
                    }
                    
                    filtered_view.push(*window_id);
                }
            }
            
            filtered_view
        }; // layout lock released here
        
        drop(filter); // Release filter lock
        
        wm_info!("Writing filtered view with {} windows", filtered_view.len());
        let mut layout = self.state.write_layout();
        layout.filtered_view = filtered_view;
        wm_info!("Applied filter: {} windows in filtered view", layout.filtered_view.len());
    }
    
    
    fn is_special_window_type(&self, window_id: u32) -> bool {
        // Query the window type property
        let type_cookie = xcb_util::ewmh::get_wm_window_type(&self.c, window_id);
        if let Ok(reply) = type_cookie.get_reply() {
            let atoms = reply.atoms();
            for atom in atoms {
                // Check if it's a special window type by getting atom name
                let atom_name_cookie = xcb::get_atom_name(&self.c, *atom);
                if let Ok(atom_name_reply) = atom_name_cookie.get_reply() {
                    let name = atom_name_reply.name();
                    if name.contains("_NET_WM_WINDOW_TYPE_DOCK") ||
                       name.contains("_NET_WM_WINDOW_TYPE_PANEL") ||
                       name.contains("_NET_WM_WINDOW_TYPE_DESKTOP") ||
                       name.contains("_NET_WM_WINDOW_TYPE_SPLASH") ||
                       name.contains("_NET_WM_WINDOW_TYPE_TOOLBAR") ||
                       name.contains("_NET_WM_WINDOW_TYPE_MENU") {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn should_include_window(&self, window: &CoreWindow, options: &[crate::core::types::Condition]) -> bool {
        // Check mapped_only condition
        if options.iter().any(|opt| matches!(opt, crate::core::types::Condition::MappedOnly)) {
            if !matches!(window.attrs.map_state, crate::core::types::MapState::Viewable) {
                return false;
            }
        }
        
        // Check omit_hidden condition
        if options.iter().any(|opt| matches!(opt, crate::core::types::Condition::OmitHidden)) {
            if matches!(window.attrs.map_state, crate::core::types::MapState::Unmapped) {
                return false;
            }
        }
        
        // Check no_special condition (skip panels, docks, desktop windows, etc.)
        if options.iter().any(|opt| matches!(opt, crate::core::types::Condition::NoSpecial)) {
            if window.attrs.override_redirect {
                return false;
            }
            // Also check for special window types
            if self.is_special_window_type(window.id) {
                return false;
            }
        }
        
        // Check no_override_redirect condition
        if options.iter().any(|opt| matches!(opt, crate::core::types::Condition::NoOverrideRedirect)) {
            if window.attrs.override_redirect {
                return false;
            }
        }
        
        true
    }
    
    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let layout = self.state.read_layout();
        let colored = self.state.has_option(&Condition::Colorful);
        let show_diff = self.state.has_option(&Condition::ShowDiff);
        
        for (i, wid) in layout.filtered_view.iter().enumerate() {
            // Try to get window from layout, or query directly if not found (for client windows)
            let window = layout.windows.get(wid).cloned()
                .or_else(|| self.query_window_info(*wid));
                
            if let Some(w) = window {
                if show_diff && changes.is_some() && changes.as_ref().unwrap().contains(&wid) {
                    println!("{}: {}", i, win2str(&w, colored).on_white());
                } else {
                    println!("{}: {}", i, win2str(&w, colored));
                }
            }
        }
    }
}

#[cfg(feature = "x11")]
pub fn monitor(ctx: &Context) {
    wm_info!("Starting monitor mode...");
    
    // Initial window refresh and display
    ctx.refresh_windows();
    ctx.dump_windows(None);
    
    // Set up event monitoring
    let event_mask = xproto::EVENT_MASK_STRUCTURE_NOTIFY 
        | xproto::EVENT_MASK_PROPERTY_CHANGE
        | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY;
    
    // Change window attributes to receive events on root window
    let change_attrs = xcb::change_window_attributes(
        ctx.c,
        ctx.root,
        &[(xproto::CW_EVENT_MASK, event_mask)]
    );
    
    if change_attrs.request_check().is_err() {
        wm_info!("Failed to set event mask");
        return;
    }
    
    wm_info!("Monitor mode active - watching for window events...");
    
    // Event loop
    loop {
        match ctx.c.wait_for_event() {
            Some(event) => {
                let r = event.response_type() & !0x80;
                match r {
                    xproto::CONFIGURE_NOTIFY => {
                        let notify: &xproto::ConfigureNotifyEvent = unsafe { xcb::cast_event(&event) };
                        wm_info!("Window configured: 0x{:x} {}x{}+{}+{}", 
                               notify.window(), notify.width(), notify.height(), 
                               notify.x(), notify.y());
                        ctx.refresh_windows();
                        ctx.dump_windows(None);
                    },
                    xproto::MAP_NOTIFY => {
                        let notify: &xproto::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                        wm_info!("Window mapped: 0x{:x}", notify.window());
                        ctx.refresh_windows();
                        ctx.dump_windows(None);
                    },
                    xproto::UNMAP_NOTIFY => {
                        let notify: &xproto::UnmapNotifyEvent = unsafe { xcb::cast_event(&event) };
                        wm_info!("Window unmapped: 0x{:x}", notify.window());
                        ctx.refresh_windows();
                        ctx.dump_windows(None);
                    },
                    xproto::CREATE_NOTIFY => {
                        let notify: &xproto::CreateNotifyEvent = unsafe { xcb::cast_event(&event) };
                        wm_info!("Window created: 0x{:x}", notify.window());
                        ctx.refresh_windows();
                        ctx.dump_windows(None);
                    },
                    xproto::DESTROY_NOTIFY => {
                        let notify: &xproto::DestroyNotifyEvent = unsafe { xcb::cast_event(&event) };
                        wm_info!("Window destroyed: 0x{:x}", notify.window());
                        ctx.refresh_windows();
                        ctx.dump_windows(None);
                    },
                    _ => {
                        // Other events we don't handle yet
                    }
                }
            },
            None => {
                wm_info!("X11 connection lost, exiting monitor mode");
                break;
            }
        }
    }
    
    wm_info!("Monitor mode ended");
}

#[cfg(not(feature = "x11"))]
pub fn x11_specific_functionality() {
    // placeholder for non-X11 builds
}

