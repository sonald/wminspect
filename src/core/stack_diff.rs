use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use crate::core::types::{WindowId, WindowStackView};

/// A cached stack difference calculator to optimize query_tree performance
#[derive(Debug, Clone)]
pub struct CachedStackDiff {
    /// Cache of previous stack state (window_id -> stack_position)
    previous_stack: HashMap<WindowId, usize>,
    /// Hash of the previous stack for quick comparison
    previous_hash: u64,
    /// Cache of computed differences
    diff_cache: HashMap<u64, StackDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackDiff {
    /// Windows that were added
    pub added: Vec<WindowId>,
    /// Windows that were removed
    pub removed: Vec<WindowId>,
    /// Windows that changed position (window_id, old_pos, new_pos)
    pub moved: Vec<(WindowId, usize, usize)>,
    /// Windows that remain in the same position
    pub unchanged: Vec<WindowId>,
}

impl Default for CachedStackDiff {
    fn default() -> Self {
        Self::new()
    }
}

impl CachedStackDiff {
    pub fn new() -> Self {
        Self {
            previous_stack: HashMap::new(),
            previous_hash: 0,
            diff_cache: HashMap::new(),
        }
    }

    /// Calculate hash for a stack view
    fn calculate_stack_hash(stack: &WindowStackView) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        stack.hash(&mut hasher);
        hasher.finish()
    }

    /// Compute diff between current and previous stack
    pub fn compute_diff(&mut self, current_stack: &WindowStackView) -> StackDiff {
        let current_hash = Self::calculate_stack_hash(current_stack);
        
        // Check if we already have this diff cached
        if let Some(cached_diff) = self.diff_cache.get(&current_hash) {
            return cached_diff.clone();
        }

        // If this is the first time or hash matches previous, return empty diff
        if self.previous_stack.is_empty() || current_hash == self.previous_hash {
            let diff = StackDiff {
                added: Vec::new(),
                removed: Vec::new(),
                moved: Vec::new(),
                unchanged: current_stack.clone(),
            };
            self.update_cache(current_stack, current_hash, diff.clone());
            return diff;
        }

        // Create current stack position map
        let current_map: HashMap<WindowId, usize> = current_stack
            .iter()
            .enumerate()
            .map(|(pos, &window_id)| (window_id, pos))
            .collect();

        let mut diff = StackDiff {
            added: Vec::new(),
            removed: Vec::new(),
            moved: Vec::new(),
            unchanged: Vec::new(),
        };

        // Find added and moved windows
        for (pos, &window_id) in current_stack.iter().enumerate() {
            if let Some(&prev_pos) = self.previous_stack.get(&window_id) {
                if pos == prev_pos {
                    diff.unchanged.push(window_id);
                } else {
                    diff.moved.push((window_id, prev_pos, pos));
                }
            } else {
                diff.added.push(window_id);
            }
        }

        // Find removed windows (maintain original order)
        let mut removed_windows: Vec<(WindowId, usize)> = Vec::new();
        for (&window_id, &pos) in &self.previous_stack {
            if !current_map.contains_key(&window_id) {
                removed_windows.push((window_id, pos));
            }
        }
        
        // Sort by position to maintain order
        removed_windows.sort_by_key(|&(_, pos)| pos);
        diff.removed = removed_windows.into_iter().map(|(id, _)| id).collect();

        // Update cache
        self.update_cache(current_stack, current_hash, diff.clone());
        diff
    }

    /// Update internal cache with new state
    fn update_cache(&mut self, current_stack: &WindowStackView, current_hash: u64, diff: StackDiff) {
        self.previous_stack = current_stack
            .iter()
            .enumerate()
            .map(|(pos, &window_id)| (window_id, pos))
            .collect();
        self.previous_hash = current_hash;
        
        // Limit cache size to prevent memory growth
        if self.diff_cache.len() > 100 {
            self.diff_cache.clear();
        }
        self.diff_cache.insert(current_hash, diff);
    }

    /// Clear all cached data
    pub fn clear(&mut self) {
        self.previous_stack.clear();
        self.previous_hash = 0;
        self.diff_cache.clear();
    }

    /// Get cache statistics for debugging
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.previous_stack.len(), self.diff_cache.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_diff() {
        let mut diff_calc = CachedStackDiff::new();
        let stack = vec![1, 2, 3];
        let diff = diff_calc.compute_diff(&stack);
        
        assert_eq!(diff.unchanged, stack);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.moved.is_empty());
    }

    #[test]
    fn test_added_windows() {
        let mut diff_calc = CachedStackDiff::new();
        let initial_stack = vec![1, 2, 3];
        let _ = diff_calc.compute_diff(&initial_stack);
        
        let new_stack = vec![1, 2, 3, 4, 5];
        let diff = diff_calc.compute_diff(&new_stack);
        
        assert_eq!(diff.added, vec![4, 5]);
        assert_eq!(diff.unchanged, vec![1, 2, 3]);
        assert!(diff.removed.is_empty());
        assert!(diff.moved.is_empty());
    }

    #[test]
    fn test_removed_windows() {
        let mut diff_calc = CachedStackDiff::new();
        let initial_stack = vec![1, 2, 3, 4, 5];
        let _ = diff_calc.compute_diff(&initial_stack);
        
        let new_stack = vec![1, 3, 5];
        let diff = diff_calc.compute_diff(&new_stack);
        
        assert_eq!(diff.removed, vec![2, 4]);
        assert_eq!(diff.unchanged, vec![1]); // Only window 1 stays in same position (0)
        assert!(diff.added.is_empty());
        // Windows 3 and 5 moved positions: 3 from pos 2 to 1, 5 from pos 4 to 2
        assert_eq!(diff.moved.len(), 2);
        assert!(diff.moved.contains(&(3, 2, 1)));
        assert!(diff.moved.contains(&(5, 4, 2)));
    }

    #[test]
    fn test_moved_windows() {
        let mut diff_calc = CachedStackDiff::new();
        let initial_stack = vec![1, 2, 3, 4];
        let _ = diff_calc.compute_diff(&initial_stack);
        
        let new_stack = vec![4, 2, 1, 3];
        let diff = diff_calc.compute_diff(&new_stack);
        
        assert_eq!(diff.moved.len(), 3);
        assert!(diff.moved.contains(&(4, 3, 0)));
        assert!(diff.moved.contains(&(1, 0, 2)));
        assert!(diff.moved.contains(&(3, 2, 3)));
        assert_eq!(diff.unchanged, vec![2]);
    }

    #[test]
    fn test_cache_hit() {
        let mut diff_calc = CachedStackDiff::new();
        let stack = vec![1, 2, 3];
        
        // First computation
        let diff1 = diff_calc.compute_diff(&stack);
        
        // Second computation with same stack should hit cache
        let diff2 = diff_calc.compute_diff(&stack);
        
        assert_eq!(diff1, diff2);
        assert_eq!(diff_calc.cache_stats().1, 1); // One cached result
    }
}
