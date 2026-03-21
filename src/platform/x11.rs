/// x11-specific functionality
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
fn geometry_from_components(
    relative_x: i16,
    relative_y: i16,
    width: u16,
    height: u16,
    absolute_position: Option<(i16, i16)>,
) -> Geometry {
    let (x, y) = absolute_position.unwrap_or((relative_x, relative_y));

    Geometry {
        x,
        y,
        width,
        height,
    }
}

#[cfg(feature = "x11")]
pub struct Context<'a> {
    pub c: &'a ewmh::Connection,
    pub root: u32,
    state: StateRef,
    formatter: crate::core::colorized_output::ColorizedFormatter,
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
            formatter: crate::core::colorized_output::ColorizedFormatter::new(),
        }
    }

    pub fn new_with_formatter(
        c: &'a ewmh::Connection,
        f: Filter,
        formatter: crate::core::colorized_output::ColorizedFormatter,
    ) -> Context<'a> {
        let screen = c.get_setup().roots().next().unwrap();
        let state = create_state_ref(f);

        Context {
            c,
            root: screen.root(),
            state,
            formatter,
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

    pub fn set_show_sequence_numbers(&mut self) {
        self.state.add_option(Condition::ShowSequenceNumbers);
    }

    /// Check if a window should be included based on command-line conditions
    fn check_window_conditions(&self, window: &CoreWindow, attrs: &Attributes) -> bool {
        // Check MappedOnly condition - only show mapped windows
        if self.state.has_option(&Condition::MappedOnly) && attrs.map_state != MapState::Viewable {
            return false;
        }

        // Check OmitHidden condition - exclude hidden/iconified windows
        if self.state.has_option(&Condition::OmitHidden) && attrs.map_state == MapState::Unviewable
        {
            return false;
        }

        // Check NoSpecial condition - ignore special windows (docks, panels, etc.)
        if self.state.has_option(&Condition::NoSpecial)
            && (self.is_special_window(window, attrs) || self.is_special_window_type(window.id))
        {
            return false;
        }

        // Check ClientsOnly condition - only include client-managed windows
        if self.state.has_option(&Condition::ClientsOnly) && attrs.override_redirect {
            return false;
        }

        // Check NoOverrideRedirect condition - ignore override-redirect windows
        if self.state.has_option(&Condition::NoOverrideRedirect) && attrs.override_redirect {
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
        if name_lower.contains("dock")
            || name_lower.contains("panel")
            || name_lower.contains("toolbar")
            || name_lower.contains("desktop")
            || name_lower.contains("notification")
            || name_lower.starts_with("xfce4-")
            || name_lower.starts_with("gnome-")
        {
            return true;
        }

        false
    }

    /// Get client window list from the window manager.
    fn get_client_windows(&self) -> Vec<u32> {
        ewmh::get_client_list(self.c, 0)
            .get_reply()
            .map(|reply| reply.windows().to_vec())
            .unwrap_or_default()
    }

    /// Detect special windows by EWMH type metadata.
    fn is_special_window_type(&self, window_id: u32) -> bool {
        let Ok(reply) = ewmh::get_wm_window_type(self.c, window_id).get_reply() else {
            return false;
        };

        for atom in reply.atoms() {
            let Ok(atom_name_reply) = xproto::get_atom_name(self.c, *atom).get_reply() else {
                continue;
            };
            let name = atom_name_reply.name();
            if name.contains("_NET_WM_WINDOW_TYPE_DOCK")
                || name.contains("_NET_WM_WINDOW_TYPE_PANEL")
                || name.contains("_NET_WM_WINDOW_TYPE_DESKTOP")
                || name.contains("_NET_WM_WINDOW_TYPE_SPLASH")
                || name.contains("_NET_WM_WINDOW_TYPE_TOOLBAR")
                || name.contains("_NET_WM_WINDOW_TYPE_MENU")
            {
                return true;
            }
        }

        false
    }

    pub fn refresh_windows(&self) {
        wm_info!("Refreshing windows...");

        let tree_cookie = xproto::query_tree(self.c, self.root);
        let tree = match tree_cookie.get_reply() {
            Ok(tree) => tree,
            Err(e) => {
                wm_trace!("Failed to query window tree: {}", e);
                return;
            }
        };
        let children = tree.children();

        let mut windows = Vec::new();
        for win in children {
            if let Some(window) = self.fetch_window_info(*win) {
                windows.push(window);
            }
        }

        {
            let mut layout = self.state.write_layout();
            layout.clear();
            layout.stack_view = windows.iter().map(|w| w.id).collect();
            for window in windows {
                layout.insert_window(window);
            }
        }

        self.apply_filter();

        let layout = self.state.read_layout();
        wm_info!(
            "Refreshed {} windows, {} passed filter",
            layout.window_count(),
            layout.filtered_view.len()
        );
    }

    fn apply_filter(&self) {
        let filter = self.state.read_filter();
        let clients_only = self.state.has_option(&Condition::ClientsOnly);

        let filtered_view = {
            let layout = self.state.read_layout();
            let window_ids_to_check = if clients_only {
                self.get_client_windows()
            } else {
                layout.stack_view.clone()
            };

            let mut filtered_view = Vec::new();
            for window_id in &window_ids_to_check {
                let window = layout
                    .windows
                    .get(window_id)
                    .cloned()
                    .or_else(|| self.fetch_window_info(*window_id));

                if let Some(window) = window {
                    if !self.check_window_conditions(&window, &window.attrs) {
                        continue;
                    }
                    if filter.rule_count() > 0 && !filter.apply_to(&window) {
                        continue;
                    }
                    filtered_view.push(*window_id);
                }
            }

            filtered_view
        };

        let mut layout = self.state.write_layout();
        layout.filtered_view = filtered_view;
    }

    /// Fetch complete window information with graceful error handling
    fn fetch_window_info(&self, win: u32) -> Option<CoreWindow> {
        let attrs_cookie = xproto::get_window_attributes(self.c, win);
        let geom_cookie = xproto::get_geometry(self.c, win);
        let translate_cookie = xproto::translate_coordinates(self.c, win, self.root, 0, 0);

        let attrs_result = attrs_cookie.get_reply();
        let geom_result = geom_cookie.get_reply();
        let translate_result = translate_cookie.get_reply();

        let mut has_critical_data = true;
        if let Err(e) = &attrs_result {
            wm_trace!("Failed to get attributes for window 0x{:x}: {}", win, e);
            has_critical_data = false;
        }
        if let Err(e) = &geom_result {
            wm_trace!("Failed to get geometry for window 0x{:x}: {}", win, e);
            has_critical_data = false;
        }
        if let Err(e) = &translate_result {
            wm_trace!(
                "Failed to translate coordinates for window 0x{:x}, falling back to relative geometry: {}",
                win,
                e
            );
        }

        if !has_critical_data {
            return None;
        }

        let attrs = attrs_result.ok()?;
        let geom = geom_result.ok()?;
        let name = self.get_window_name(win).unwrap_or_default();
        let absolute_position = translate_result.ok().and_then(|reply| {
            if reply.same_screen() {
                Some((reply.dst_x(), reply.dst_y()))
            } else {
                None
            }
        });

        let attributes = self.map_attributes(&attrs);
        let geometry = self.map_geometry(&geom, absolute_position);

        Some(CoreWindow {
            id: win,
            name,
            attrs: attributes,
            geom: geometry,
            valid: true,
        })
    }

    fn get_window_name(&self, window_id: u32) -> Option<String> {
        self.get_ewmh_window_name(window_id)
            .or_else(|| self.get_wm_name(window_id))
    }

    fn get_ewmh_window_name(&self, window_id: u32) -> Option<String> {
        ewmh::get_wm_name(self.c, window_id)
            .get_reply()
            .ok()
            .map(|reply| reply.string().to_string())
    }

    fn get_wm_name(&self, window_id: u32) -> Option<String> {
        let reply = xproto::get_property(
            self.c,
            false,
            window_id,
            xproto::ATOM_WM_NAME,
            xproto::ATOM_STRING,
            0,
            1024,
        )
        .get_reply()
        .ok()?;

        if reply.value_len() == 0 {
            return None;
        }

        String::from_utf8(reply.value().to_vec()).ok()
    }

    /// Map X11 attributes to internal representation with proper defaults
    fn map_attributes(&self, attrs: &xproto::GetWindowAttributesReply) -> Attributes {
        Attributes {
            override_redirect: attrs.override_redirect(),
            map_state: match attrs.map_state() as u32 {
                xproto::MAP_STATE_UNMAPPED => MapState::Unmapped,
                xproto::MAP_STATE_UNVIEWABLE => MapState::Unviewable,
                _ => MapState::Viewable,
            },
        }
    }

    /// Map X11 geometry to internal representation
    fn map_geometry(
        &self,
        geom: &xproto::GetGeometryReply,
        absolute_position: Option<(i16, i16)>,
    ) -> Geometry {
        geometry_from_components(
            geom.x(),
            geom.y(),
            geom.width(),
            geom.height(),
            absolute_position,
        )
    }

    pub fn dump_windows(&self, changes: Option<WindowListView>) {
        let layout = self.state.read_layout();
        let show_diff = self.state.has_option(&Condition::ShowDiff);

        for (i, wid) in layout.filtered_view.iter().enumerate() {
            let window = layout
                .windows
                .get(wid)
                .cloned()
                .or_else(|| self.fetch_window_info(*wid));

            if let Some(w) = window {
                let geom_str = format!("{}", w.geom);
                let attrs_str = format!("{}", w.attrs);
                let is_diff =
                    show_diff && changes.is_some() && changes.as_ref().unwrap().contains(wid);
                let formatted_output = self
                    .formatter
                    .format_window_entry(i, w.id, &w.name, &geom_str, &attrs_str, is_diff);
                println!("{}", formatted_output);
            }
        }
    }
}

#[cfg(feature = "x11")]
pub fn monitor(ctx: &Context) {
    wm_info!("Starting monitor mode...");
    let show_sequence = ctx.state.has_option(&Condition::ShowSequenceNumbers);
    let run_once = std::env::var_os("WMINSPECT_MONITOR_ONCE").is_some();
    let mut event_count = 0u32;

    if show_sequence {
        println!("Event #{}: Initial window state", event_count);
    }
    ctx.refresh_windows();
    ctx.dump_windows(None);
    if run_once {
        wm_info!("Monitor mode single-shot run completed");
        return;
    }

    let event_mask = xproto::EVENT_MASK_STRUCTURE_NOTIFY
        | xproto::EVENT_MASK_PROPERTY_CHANGE
        | xproto::EVENT_MASK_SUBSTRUCTURE_NOTIFY;

    let change_attrs =
        xcb::change_window_attributes(ctx.c, ctx.root, &[(xproto::CW_EVENT_MASK, event_mask)]);
    if change_attrs.request_check().is_err() {
        wm_info!("Failed to set event mask");
        return;
    }

    wm_info!("Monitor mode active - watching for window events...");

    loop {
        match ctx.c.wait_for_event() {
            Some(event) => {
                let description = match event.response_type() & !0x80 {
                    xproto::CONFIGURE_NOTIFY => {
                        let notify: &xproto::ConfigureNotifyEvent =
                            unsafe { xcb::cast_event(&event) };
                        Some(format!(
                            "Window configured: 0x{:x} {}x{}+{}+{}",
                            notify.window(),
                            notify.width(),
                            notify.height(),
                            notify.x(),
                            notify.y()
                        ))
                    }
                    xproto::MAP_NOTIFY => {
                        let notify: &xproto::MapNotifyEvent = unsafe { xcb::cast_event(&event) };
                        Some(format!("Window mapped: 0x{:x}", notify.window()))
                    }
                    xproto::UNMAP_NOTIFY => {
                        let notify: &xproto::UnmapNotifyEvent = unsafe { xcb::cast_event(&event) };
                        Some(format!("Window unmapped: 0x{:x}", notify.window()))
                    }
                    xproto::CREATE_NOTIFY => {
                        let notify: &xproto::CreateNotifyEvent = unsafe { xcb::cast_event(&event) };
                        Some(format!("Window created: 0x{:x}", notify.window()))
                    }
                    xproto::DESTROY_NOTIFY => {
                        let notify: &xproto::DestroyNotifyEvent =
                            unsafe { xcb::cast_event(&event) };
                        Some(format!("Window destroyed: 0x{:x}", notify.window()))
                    }
                    xproto::PROPERTY_NOTIFY => {
                        let notify: &xproto::PropertyNotifyEvent =
                            unsafe { xcb::cast_event(&event) };
                        Some(format!("Window property changed: 0x{:x}", notify.window()))
                    }
                    _ => None,
                };

                if let Some(description) = description {
                    event_count += 1;
                    wm_trace!("event received: {}", description);
                    if show_sequence {
                        println!("Event #{}: {}", event_count, description);
                    }
                    ctx.refresh_windows();
                    ctx.dump_windows(None);
                }
            }
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

#[cfg(all(test, feature = "x11"))]
mod tests {
    use super::*;
    use crate::core::types::{Attributes, Geometry, MapState};
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
            if self.has_option(&Condition::MappedOnly) && attrs.map_state != MapState::Viewable {
                return false;
            }

            // Check OmitHidden condition - exclude hidden/iconified windows
            if self.has_option(&Condition::OmitHidden) && attrs.map_state == MapState::Unviewable {
                return false;
            }

            // Check NoSpecial condition - ignore special windows (docks, panels, etc.)
            if self.has_option(&Condition::NoSpecial) && self.is_special_window(window, attrs) {
                return false;
            }

            // Check ClientsOnly condition - only include client-managed windows
            if self.has_option(&Condition::ClientsOnly) && attrs.override_redirect {
                return false;
            }

            // Check NoOverrideRedirect condition - exclude override-redirect windows (popups, tooltips)
            if self.has_option(&Condition::NoOverrideRedirect) && attrs.override_redirect {
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
            if name_lower.contains("dock")
                || name_lower.contains("panel")
                || name_lower.contains("toolbar")
                || name_lower.contains("desktop")
                || name_lower.contains("notification")
                || name_lower.starts_with("xfce4-")
                || name_lower.starts_with("gnome-")
            {
                return true;
            }

            false
        }
    }

    fn create_test_window(
        name: &str,
        map_state: MapState,
        override_redirect: bool,
        width: u16,
        height: u16,
    ) -> (CoreWindow, Attributes) {
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
    fn test_geometry_prefers_absolute_position() {
        let geometry = geometry_from_components(0, 0, 400, 300, Some((120, 240)));

        assert_eq!(geometry.x, 120);
        assert_eq!(geometry.y, 240);
        assert_eq!(geometry.width, 400);
        assert_eq!(geometry.height, 300);
    }

    #[test]
    fn test_geometry_falls_back_to_relative_position() {
        let geometry = geometry_from_components(16, 32, 400, 300, None);

        assert_eq!(geometry.x, 16);
        assert_eq!(geometry.y, 32);
        assert_eq!(geometry.width, 400);
        assert_eq!(geometry.height, 300);
    }

    #[test]
    fn test_mapped_only_condition() {
        let ctx = TestContext::new();

        // Add MappedOnly condition
        ctx.add_condition(Condition::MappedOnly);

        // Test mapped window - should be included
        let (mapped_window, mapped_attrs) =
            create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&mapped_window, &mapped_attrs));

        // Test unmapped window - should be excluded
        let (unmapped_window, unmapped_attrs) =
            create_test_window("test", MapState::Unmapped, false, 100, 100);
        assert!(!ctx.check_window_conditions(&unmapped_window, &unmapped_attrs));
    }

    #[test]
    fn test_omit_hidden_condition() {
        let ctx = TestContext::new();

        // Add OmitHidden condition
        ctx.add_condition(Condition::OmitHidden);

        // Test viewable window - should be included
        let (viewable_window, viewable_attrs) =
            create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&viewable_window, &viewable_attrs));

        // Test unviewable (hidden) window - should be excluded
        let (hidden_window, hidden_attrs) =
            create_test_window("test", MapState::Unviewable, false, 100, 100);
        assert!(!ctx.check_window_conditions(&hidden_window, &hidden_attrs));
    }

    #[test]
    fn test_clients_only_condition() {
        let ctx = TestContext::new();

        // Add ClientsOnly condition
        ctx.add_condition(Condition::ClientsOnly);

        // Test client window (not override-redirect) - should be included
        let (client_window, client_attrs) =
            create_test_window("test", MapState::Viewable, false, 100, 100);
        assert!(ctx.check_window_conditions(&client_window, &client_attrs));

        // Test override-redirect window - should be excluded
        let (popup_window, popup_attrs) =
            create_test_window("test", MapState::Viewable, true, 100, 100);
        assert!(!ctx.check_window_conditions(&popup_window, &popup_attrs));
    }

    #[test]
    fn test_no_special_condition() {
        let ctx = TestContext::new();

        // Add NoSpecial condition
        ctx.add_condition(Condition::NoSpecial);

        // Test normal window - should be included
        let (normal_window, normal_attrs) =
            create_test_window("firefox", MapState::Viewable, false, 800, 600);
        assert!(ctx.check_window_conditions(&normal_window, &normal_attrs));

        // Test dock window - should be excluded
        let (dock_window, dock_attrs) =
            create_test_window("xfce4-panel", MapState::Viewable, false, 800, 30);
        assert!(!ctx.check_window_conditions(&dock_window, &dock_attrs));

        // Test override-redirect window - should be excluded
        let (or_window, or_attrs) =
            create_test_window("tooltip", MapState::Viewable, true, 100, 50);
        assert!(!ctx.check_window_conditions(&or_window, &or_attrs));

        // Test tiny window - should be excluded
        let (tiny_window, tiny_attrs) =
            create_test_window("tracking", MapState::Viewable, false, 1, 1);
        assert!(!ctx.check_window_conditions(&tiny_window, &tiny_attrs));
    }

    #[test]
    fn test_special_window_detection() {
        let ctx = TestContext::new();

        // Test various special window patterns
        let test_cases = vec![
            ("xfce4-panel", MapState::Viewable, false, 800, 30, true), // XFCE panel
            ("gnome-shell", MapState::Viewable, false, 1920, 1080, true), // GNOME shell
            ("dock", MapState::Viewable, false, 60, 800, true),        // Dock
            ("Desktop", MapState::Viewable, false, 1920, 1080, true),  // Desktop
            ("notification", MapState::Viewable, false, 300, 100, true), // Notification
            ("tooltip", MapState::Viewable, true, 100, 50, true),      // Override-redirect
            ("tracking", MapState::Viewable, false, 1, 1, true),       // Tiny window
            ("firefox", MapState::Viewable, false, 800, 600, false),   // Normal window
            ("text-editor", MapState::Viewable, false, 600, 400, false), // Normal window
        ];

        for (name, map_state, or, width, height, should_be_special) in test_cases {
            let (window, attrs) = create_test_window(name, map_state, or, width, height);
            let is_special = ctx.is_special_window(&window, &attrs);
            assert_eq!(
                is_special, should_be_special,
                "Window '{}' special detection failed: expected {}, got {}",
                name, should_be_special, is_special
            );
        }
    }

    #[test]
    fn test_multiple_conditions() {
        let ctx = TestContext::new();

        // Add multiple conditions
        ctx.add_condition(Condition::MappedOnly);
        ctx.add_condition(Condition::ClientsOnly);

        // Test window that meets both conditions - should be included
        let (good_window, good_attrs) =
            create_test_window("firefox", MapState::Viewable, false, 800, 600);
        assert!(ctx.check_window_conditions(&good_window, &good_attrs));

        // Test unmapped client window - should be excluded (fails MappedOnly)
        let (unmapped_window, unmapped_attrs) =
            create_test_window("firefox", MapState::Unmapped, false, 800, 600);
        assert!(!ctx.check_window_conditions(&unmapped_window, &unmapped_attrs));

        // Test mapped override-redirect window - should be excluded (fails ClientsOnly)
        let (popup_window, popup_attrs) =
            create_test_window("popup", MapState::Viewable, true, 200, 100);
        assert!(!ctx.check_window_conditions(&popup_window, &popup_attrs));
    }

    #[test]
    fn test_no_override_redirect_condition() {
        let ctx = TestContext::new();
        ctx.add_condition(Condition::NoOverrideRedirect);

        // Test normal client window (not override-redirect) - should be included
        let (client_window, client_attrs) =
            create_test_window("firefox", MapState::Viewable, false, 800, 600);
        assert!(ctx.check_window_conditions(&client_window, &client_attrs));

        // Test override-redirect window (popup) - should be excluded
        let (popup_window, popup_attrs) =
            create_test_window("popup", MapState::Viewable, true, 200, 100);
        assert!(!ctx.check_window_conditions(&popup_window, &popup_attrs));

        // Test tooltip (override-redirect) - should be excluded
        let (tooltip_window, tooltip_attrs) =
            create_test_window("tooltip", MapState::Viewable, true, 150, 30);
        assert!(!ctx.check_window_conditions(&tooltip_window, &tooltip_attrs));
    }

    #[test]
    fn test_show_sequence_numbers_condition() {
        let ctx = TestContext::new();

        // Test that the condition can be added without errors
        ctx.add_condition(Condition::ShowSequenceNumbers);

        // ShowSequenceNumbers doesn't affect window filtering, so any window should pass
        let (window, attrs) = create_test_window("test", MapState::Viewable, false, 400, 300);
        assert!(ctx.check_window_conditions(&window, &attrs));

        // Verify the condition is stored
        let options = ctx.state.read_options();
        assert!(options.contains(&Condition::ShowSequenceNumbers));
    }
}
