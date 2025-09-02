use std::collections::HashMap;
use std::sync::Mutex;
use globset::{Glob, GlobSet, GlobSetBuilder};
use once_cell::sync::Lazy;

/// Cache for compiled glob patterns
static GLOB_CACHE: Lazy<Mutex<HashMap<String, CompiledGlob>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Wrapper for compiled glob patterns
#[derive(Debug, Clone)]
struct CompiledGlob {
    globset: GlobSet,
    #[allow(dead_code)]
    original_pattern: String,
}

/// Optimized wildcard matcher using pre-compiled patterns
pub struct OptimizedWildcardMatcher;

impl OptimizedWildcardMatcher {
    /// Match a string against a pattern with caching
    pub fn match_pattern(pattern: &str, text: &str) -> bool {
        // Handle edge cases
        if pattern.is_empty() {
            return text.is_empty();
        }
        
        // For simple patterns without wildcards, use direct comparison
        if !Self::is_wildcard_pattern(pattern) {
            return text.contains(pattern);
        }

        // Try to get from cache first
        let compiled_glob = {
            let mut cache = GLOB_CACHE.lock().unwrap();
            
            if let Some(cached) = cache.get(pattern) {
                cached.clone()
            } else {
                // Compile new pattern
                match Self::compile_pattern(pattern) {
                    Ok(compiled) => {
                        cache.insert(pattern.to_string(), compiled.clone());
                        compiled
                    }
                    Err(_) => {
                        // Fallback to original implementation for invalid patterns
                        return crate::dsl::filter::wild_match(pattern, text);
                    }
                }
            }
        };

        compiled_glob.globset.is_match(text)
    }

    /// Check if pattern contains wildcard characters
    fn is_wildcard_pattern(pattern: &str) -> bool {
        pattern.chars().any(|c| c == '*' || c == '?')
    }

    /// Compile a wildcard pattern into a GlobSet
    fn compile_pattern(pattern: &str) -> Result<CompiledGlob, Box<dyn std::error::Error>> {
        let mut builder = GlobSetBuilder::new();
        
        // Convert shell-style wildcards to glob patterns
        let glob_pattern = Self::shell_to_glob_pattern(pattern);
        let glob = Glob::new(&glob_pattern)?;
        builder.add(glob);
        
        let globset = builder.build()?;
        
        Ok(CompiledGlob {
            globset,
            original_pattern: pattern.to_string(),
        })
    }

    /// Convert shell-style wildcards to glob patterns
    fn shell_to_glob_pattern(pattern: &str) -> String {
        // For most cases, the pattern is already in glob format
        // We just need to handle special cases
        pattern.to_string()
    }

    /// Clear the pattern cache (useful for testing and memory management)
    pub fn clear_cache() {
        let mut cache = GLOB_CACHE.lock().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats() -> usize {
        let cache = GLOB_CACHE.lock().unwrap();
        cache.len()
    }

    /// Batch match multiple patterns against a single text
    pub fn batch_match(patterns: &[&str], text: &str) -> Vec<bool> {
        patterns.iter().map(|pattern| Self::match_pattern(pattern, text)).collect()
    }

    /// Create a matcher for multiple patterns that can be reused
    pub fn create_batch_matcher(patterns: &[&str]) -> Result<BatchMatcher, Box<dyn std::error::Error>> {
        let mut builder = GlobSetBuilder::new();
        
        for pattern in patterns {
            if Self::is_wildcard_pattern(pattern) {
                let glob_pattern = Self::shell_to_glob_pattern(pattern);
                let glob = Glob::new(&glob_pattern)?;
                builder.add(glob);
            } else {
                // For non-wildcard patterns, create a simple contains check
                let glob = Glob::new(&format!("*{}*", pattern))?;
                builder.add(glob);
            }
        }
        
        let globset = builder.build()?;
        
        Ok(BatchMatcher {
            globset,
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
        })
    }
}

/// A batch matcher for multiple patterns
pub struct BatchMatcher {
    globset: GlobSet,
    patterns: Vec<String>,
}

impl BatchMatcher {
    /// Match text against all patterns and return which ones matched
    pub fn match_all(&self, text: &str) -> Vec<usize> {
        self.globset.matches(text)
    }

    /// Check if text matches any pattern
    pub fn matches_any(&self, text: &str) -> bool {
        self.globset.is_match(text)
    }

    /// Get the patterns this matcher was created with
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_wildcard() {
        assert!(OptimizedWildcardMatcher::match_pattern("test*", "test123"));
        assert!(OptimizedWildcardMatcher::match_pattern("*test", "123test"));
        assert!(!OptimizedWildcardMatcher::match_pattern("test*", "example"));
    }

    #[test]
    fn test_question_mark_wildcard() {
        assert!(OptimizedWildcardMatcher::match_pattern("t?st", "test"));
        assert!(OptimizedWildcardMatcher::match_pattern("t?st", "tast"));
        assert!(!OptimizedWildcardMatcher::match_pattern("t?st", "toast"));
    }

    #[test]
    fn test_complex_patterns() {
        assert!(OptimizedWildcardMatcher::match_pattern("*t*st*", "testing"));
        assert!(OptimizedWildcardMatcher::match_pattern("a*b*c", "aabbcc"));
        assert!(!OptimizedWildcardMatcher::match_pattern("a*b*c", "aabbdd"));
    }

    #[test]
    fn test_non_wildcard_patterns() {
        assert!(OptimizedWildcardMatcher::match_pattern("test", "test123"));
        assert!(OptimizedWildcardMatcher::match_pattern("test", "123test456"));
        assert!(!OptimizedWildcardMatcher::match_pattern("test", "example"));
    }

    #[test]
    fn test_batch_matching() {
        let patterns = vec!["test*", "*example*", "exact"];
        let results = OptimizedWildcardMatcher::batch_match(&patterns, "test123");
        assert_eq!(results, vec![true, false, false]);

        let results = OptimizedWildcardMatcher::batch_match(&patterns, "example_text");
        assert_eq!(results, vec![false, true, false]);
    }

    #[test]
    fn test_batch_matcher() {
        let patterns = vec!["test*", "*example*", "exact"];
        let matcher = OptimizedWildcardMatcher::create_batch_matcher(&patterns).unwrap();
        
        assert!(matcher.matches_any("test123"));
        assert!(matcher.matches_any("example_text"));
        assert!(!matcher.matches_any("other"));
        
        let matches = matcher.match_all("test123");
        assert_eq!(matches, vec![0]); // First pattern matches
    }

    #[test]
    fn test_cache_behavior() {
        // Test basic cache functionality - the cache mechanism works correctly
        // Note: In parallel test execution, exact cache size assertions are unreliable
        // due to race conditions, so we focus on functional correctness
        
        // Use extremely unique patterns that are unlikely to be used by other tests
        let unique_pattern1 = "cache_behavior_test_pattern_xyz_123_unique_alpha*";
        let unique_pattern2 = "*cache_behavior_test_pattern_xyz_456_unique_beta";
        
        // Test that wildcard patterns work correctly (cache is internal implementation detail)
        let result1 = OptimizedWildcardMatcher::match_pattern(unique_pattern1, "cache_behavior_test_pattern_xyz_123_unique_alpha_match");
        assert!(result1, "First wildcard pattern should match correctly");
        
        // Test pattern reuse - should work correctly regardless of cache state
        let result2 = OptimizedWildcardMatcher::match_pattern(unique_pattern1, "cache_behavior_test_pattern_xyz_123_unique_alpha_different");
        assert!(result2, "Reusing same wildcard pattern should match correctly");
        
        // Test different pattern - should also work correctly
        let result3 = OptimizedWildcardMatcher::match_pattern(unique_pattern2, "match_cache_behavior_test_pattern_xyz_456_unique_beta");
        assert!(result3, "Different wildcard pattern should match correctly");
        
        // Verify the cache is being used (non-empty after wildcard operations)
        let final_cache_size = OptimizedWildcardMatcher::cache_stats();
        assert!(final_cache_size > 0, "Cache should contain entries after wildcard pattern operations");
        
        // Test that non-wildcard patterns work (these bypass cache)
        let result4 = OptimizedWildcardMatcher::match_pattern("exact_match", "exact_match");
        assert!(result4, "Exact pattern matching should work correctly");
        
        let result5 = OptimizedWildcardMatcher::match_pattern("no_match", "different");
        assert!(!result5, "Non-matching patterns should return false correctly");
    }

    #[test]
    fn test_edge_cases() {
        assert!(OptimizedWildcardMatcher::match_pattern("*", "anything"));
        assert!(OptimizedWildcardMatcher::match_pattern("", ""));
        assert!(!OptimizedWildcardMatcher::match_pattern("test", ""));
        assert!(!OptimizedWildcardMatcher::match_pattern("", "test"));
    }
}
