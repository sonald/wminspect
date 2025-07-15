use wminspect::dsl::filter::{FilterItem, FilterRule, Action, Predicate, Op, Matcher};
use wminspect::core::types::MapState;
use serde_json;
use bincode;

#[cfg(test)]
mod serialization_round_trip_tests {
    use super::*;

    #[test]
    fn test_json_serialization_round_trip() {
        let item = FilterItem {
            action: Action::Pin,
            rule: FilterRule::ClientsOnly,
        };
        
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: FilterItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_bincode_serialization_round_trip() {
        let item = FilterItem {
            action: Action::FilterOut,
            rule: FilterRule::Single {
                pred: Predicate::Name,
                op: Op::Eq,
                matcher: Matcher::Wildcard("test*".to_string()),
            },
        };
        
        let encoded = bincode::serialize(&item).unwrap();
        let decoded: FilterItem = bincode::deserialize(&encoded).unwrap();
        assert_eq!(item, decoded);
    }

    #[test]
    fn test_complex_rule_serialization() {
        let complex_rule = FilterRule::All(vec![
            Box::new(FilterRule::Single {
                pred: Predicate::Geom("width".to_string()),
                op: Op::GE,
                matcher: Matcher::IntegralValue(400),
            }),
            Box::new(FilterRule::Single {
                pred: Predicate::Attr("map_state".to_string()),
                op: Op::Eq,
                matcher: Matcher::MapStateValue(MapState::Viewable),
            }),
        ]);
        
        let item = FilterItem {
            action: Action::Pin,
            rule: complex_rule,
        };
        
        // Test JSON
        let json = serde_json::to_string(&item).unwrap();
        let json_decoded: FilterItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, json_decoded);
        
        // Test bincode
        let encoded = bincode::serialize(&item).unwrap();
        let bin_decoded: FilterItem = bincode::deserialize(&encoded).unwrap();
        assert_eq!(item, bin_decoded);
    }

    #[test]
    fn test_nested_rule_serialization() {
        let nested_rule = FilterRule::Any(vec![
            Box::new(FilterRule::Not(Box::new(FilterRule::ClientsOnly))),
            Box::new(FilterRule::All(vec![
                Box::new(FilterRule::Single {
                    pred: Predicate::Id,
                    op: Op::Eq,
                    matcher: Matcher::Wildcard("0x123*".to_string()),
                }),
                Box::new(FilterRule::Single {
                    pred: Predicate::Attr("override_redirect".to_string()),
                    op: Op::Eq,
                    matcher: Matcher::BoolValue(false),
                }),
            ])),
        ]);
        
        let item = FilterItem {
            action: Action::FilterOut,
            rule: nested_rule,
        };
        
        // Test JSON
        let json = serde_json::to_string(&item).unwrap();
        let json_decoded: FilterItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, json_decoded);
        
        // Test bincode
        let encoded = bincode::serialize(&item).unwrap();
        let bin_decoded: FilterItem = bincode::deserialize(&encoded).unwrap();
        assert_eq!(item, bin_decoded);
    }

    #[test]
    fn test_action_serialization() {
        // Test Pin action
        let pin_json = serde_json::to_string(&Action::Pin).unwrap();
        let pin_decoded: Action = serde_json::from_str(&pin_json).unwrap();
        assert_eq!(Action::Pin, pin_decoded);
        
        // Test FilterOut action
        let filter_json = serde_json::to_string(&Action::FilterOut).unwrap();
        let filter_decoded: Action = serde_json::from_str(&filter_json).unwrap();
        assert_eq!(Action::FilterOut, filter_decoded);
    }

    #[test]
    fn test_map_state_serialization() {
        for state in &[MapState::Viewable, MapState::Unmapped, MapState::Unviewable] {
            let json = serde_json::to_string(state).unwrap();
            let decoded: MapState = serde_json::from_str(&json).unwrap();
            assert_eq!(*state, decoded);
            
            let encoded = bincode::serialize(state).unwrap();
            let bin_decoded: MapState = bincode::deserialize(&encoded).unwrap();
            assert_eq!(*state, bin_decoded);
        }
    }
}
