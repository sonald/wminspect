use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use crate::core::types::*;
// use crate::core::error::WmError; // TODO: Remove if not needed
use crate::core::stack_diff::CachedStackDiff;
use crate::dsl::Filter;
/// Contains cached windows data, synchronized with the server
#[derive(Debug)]
pub struct WindowsLayout {
    /// Collection of window information
    pub windows: HashMap<WindowId, Window>,
    /// View maintained by stacking order (bottom -> top)
    pub stack_view: WindowStackView,
    /// Filtered view based on current rules
    pub filtered_view: WindowStackView,
    /// Set of pinned windows
    pub pinned_windows: WindowListView,
}

impl Default for WindowsLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsLayout {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            stack_view: WindowStackView::new(),
            filtered_view: WindowStackView::new(),
            pinned_windows: WindowListView::new(),
        }
    }

    pub fn clear(&mut self) {
        self.windows.clear();
        self.stack_view.clear();
        self.filtered_view.clear();
        self.pinned_windows.clear();
    }

    pub fn get_window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn insert_window(&mut self, window: Window) {
        self.windows.insert(window.id, window);
    }

    pub fn remove_window(&mut self, id: WindowId) {
        self.windows.remove(&id);
        self.stack_view.retain(|&w| w != id);
        self.filtered_view.retain(|&w| w != id);
        self.pinned_windows.retain(|&w| w != id);
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn is_window_in_filtered_view(&self, id: WindowId) -> bool {
        self.filtered_view.contains(&id)
    }

    /// Pin a window (add to pinned set)
    pub fn pin_window(&mut self, id: WindowId) {
        self.pinned_windows.insert(id);
    }

    /// Unpin a window (remove from pinned set)
    pub fn unpin_window(&mut self, id: WindowId) {
        self.pinned_windows.remove(&id);
    }

    /// Check if a window is pinned
    pub fn is_window_pinned(&self, id: WindowId) -> bool {
        self.pinned_windows.contains(&id)
    }

    /// Get all pinned windows
    pub fn get_pinned_windows(&self) -> &WindowListView {
        &self.pinned_windows
    }

    /// Toggle pin state of a window
    pub fn toggle_pin(&mut self, id: WindowId) {
        if self.is_window_pinned(id) {
            self.unpin_window(id);
        } else {
            self.pin_window(id);
        }
    }
}

/// Global application state container
#[derive(Debug)]
pub struct GlobalState {
    /// Windows layout and organization
    layout: Arc<RwLock<WindowsLayout>>,
    /// Filter rules for window management
    filter: Arc<RwLock<Filter>>,
    /// Application configuration options
    options: Arc<RwLock<Vec<Condition>>>,
    /// Flag indicating if clients need updating
    clients_pending_update: Arc<Mutex<bool>>,
    /// Cached stack diff calculator for performance
    stack_diff_cache: Arc<Mutex<CachedStackDiff>>,
}

impl GlobalState {
    pub fn new(filter: Filter) -> Self {
        Self {
            layout: Arc::new(RwLock::new(WindowsLayout::new())),
            filter: Arc::new(RwLock::new(filter)),
            options: Arc::new(RwLock::new(Vec::new())),
            clients_pending_update: Arc::new(Mutex::new(false)),
            stack_diff_cache: Arc::new(Mutex::new(CachedStackDiff::new())),
        }
    }

    /// Get a read lock on the windows layout
    pub fn read_layout(&self) -> std::sync::RwLockReadGuard<'_, WindowsLayout> {
        self.layout.read().unwrap()
    }

    /// Get a write lock on the windows layout
    pub fn write_layout(&self) -> std::sync::RwLockWriteGuard<'_, WindowsLayout> {
        self.layout.write().unwrap()
    }

    /// Get a read lock on the filter
    pub fn read_filter(&self) -> std::sync::RwLockReadGuard<'_, Filter> {
        self.filter.read().unwrap()
    }

    /// Get a write lock on the filter
    pub fn write_filter(&self) -> std::sync::RwLockWriteGuard<'_, Filter> {
        self.filter.write().unwrap()
    }

    /// Get a read lock on the options
    pub fn read_options(&self) -> std::sync::RwLockReadGuard<'_, Vec<Condition>> {
        self.options.read().unwrap()
    }

    /// Get a write lock on the options
    pub fn write_options(&self) -> std::sync::RwLockWriteGuard<'_, Vec<Condition>> {
        self.options.write().unwrap()
    }

    /// Check if a specific option is enabled
    pub fn has_option(&self, condition: &Condition) -> bool {
        self.read_options().iter().any(|c| {
            std::mem::discriminant(c) == std::mem::discriminant(condition)
        })
    }

    /// Add an option to the configuration
    pub fn add_option(&self, condition: Condition) {
        self.write_options().push(condition);
    }

    /// Set clients pending update flag
    pub fn set_clients_pending_update(&self, pending: bool) {
        *self.clients_pending_update.lock().unwrap() = pending;
    }

    /// Get clients pending update flag
    pub fn get_clients_pending_update(&self) -> bool {
        *self.clients_pending_update.lock().unwrap()
    }

    /// Clear clients pending update flag and return previous value
    pub fn clear_clients_pending_update(&self) -> bool {
        let mut guard = self.clients_pending_update.lock().unwrap();
        let prev = *guard;
        *guard = false;
        prev
    }

    /// Execute a closure with mutable access to a window
    pub fn with_window_mut<F, R>(&self, window_id: WindowId, f: F) -> Option<R>
    where
        F: FnOnce(&mut Window) -> R,
    {
        let mut layout = self.write_layout();
        layout.get_window_mut(window_id).map(f)
    }

    /// Execute a closure with read access to a window
    pub fn with_window<F, R>(&self, window_id: WindowId, f: F) -> Option<R>
    where
        F: FnOnce(&Window) -> R,
    {
        let layout = self.read_layout();
        layout.get_window(window_id).map(f)
    }

    /// Clone the current state (for testing or backup purposes)
    pub fn clone_layout(&self) -> WindowsLayout {
        let layout = self.read_layout();
        WindowsLayout {
            windows: layout.windows.clone(),
            stack_view: layout.stack_view.clone(),
            filtered_view: layout.filtered_view.clone(),
            pinned_windows: layout.pinned_windows.clone(),
        }
    }

    /// Compute stack diff for current stack view
    pub fn compute_stack_diff(&self) -> crate::core::stack_diff::StackDiff {
        let layout = self.read_layout();
        let mut cache = self.stack_diff_cache.lock().unwrap();
        cache.compute_diff(&layout.stack_view)
    }

    /// Clear stack diff cache
    pub fn clear_stack_diff_cache(&self) {
        let mut cache = self.stack_diff_cache.lock().unwrap();
        cache.clear();
    }

    /// Get stack diff cache statistics
    pub fn stack_diff_cache_stats(&self) -> (usize, usize) {
        let cache = self.stack_diff_cache.lock().unwrap();
        cache.cache_stats()
    }

    /// Pin a window by ID
    pub fn pin_window(&self, id: WindowId) {
        self.write_layout().pin_window(id);
    }

    /// Unpin a window by ID
    pub fn unpin_window(&self, id: WindowId) {
        self.write_layout().unpin_window(id);
    }

    /// Check if a window is pinned
    pub fn is_window_pinned(&self, id: WindowId) -> bool {
        self.read_layout().is_window_pinned(id)
    }

    /// Toggle pin state of a window
    pub fn toggle_pin(&self, id: WindowId) {
        self.write_layout().toggle_pin(id);
    }

    /// Get count of pinned windows
    pub fn pinned_window_count(&self) -> usize {
        self.read_layout().pinned_windows.len()
    }

    /// Get all pinned window IDs
    pub fn get_pinned_window_ids(&self) -> Vec<WindowId> {
        self.read_layout().pinned_windows.iter().copied().collect()
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self::new(Filter::new())
    }
}

/// Thread-safe reference to global state
pub type StateRef = Arc<GlobalState>;

/// Create a new shared state reference
pub fn create_state_ref(filter: Filter) -> StateRef {
    Arc::new(GlobalState::new(filter))
}
