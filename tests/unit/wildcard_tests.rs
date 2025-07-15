use wminspect::dsl::filter::wild_match;

#[cfg(test)]
mod wildcard_matching_tests {
    use super::*;

    #[test]
    fn test_wildcard_match_basic() {
        assert!(wild_match("test*", "test123"));
        assert!(wild_match("*123", "test123"));
        assert!(!wild_match("?123", "test123"));
    }

    #[test]
    fn test_wildcard_match_complex() {
        assert!(wild_match("t*t?23", "testing123"));
        assert!(wild_match("*te?t*", "xxxtestxx"));
        assert!(!wild_match("*tes?xx", "xxxtestxx"));
    }

    #[test]
    fn test_wildcard_match_edge_cases() {
        assert!(!wild_match("", "test"));
        assert!(wild_match("*", "test"));
        assert!(wild_match("test", "test"));
    }
}
