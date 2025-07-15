use wminspect::dsl::filter::{parse_rule, scan_tokens, FilterItem};

#[test]
fn test_parser_basic() {
    let mut tokens = scan_tokens("name = example");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 1);
    let serialized = serde_json::to_string(&parsed).unwrap();
    let deserialized: Vec<FilterItem> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed, deserialized);
}

#[test]
fn test_parser_complex() {
    let mut tokens = scan_tokens("all(name = test, id = 123)");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 1);
    let serialized = serde_json::to_string(&parsed).unwrap();
    let deserialized: Vec<FilterItem> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed, deserialized);
}

#[test]
fn test_parser_ast_equality() {
    let mut tokens1 = scan_tokens("name = example");
    let mut tokens2 = scan_tokens("name = example");
    
    let parsed1 = parse_rule(&mut tokens1).unwrap();
    let parsed2 = parse_rule(&mut tokens2).unwrap();
    
    assert_eq!(parsed1, parsed2);
}

#[test]
fn test_parser_with_actions() {
    let mut tokens = scan_tokens("name = test: pin");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].action, wminspect::dsl::filter::Action::Pin);
}

#[test]
fn test_parser_multiple_rules() {
    let mut tokens = scan_tokens("name = test: pin; id = 123: filter");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].action, wminspect::dsl::filter::Action::Pin);
    assert_eq!(parsed[1].action, wminspect::dsl::filter::Action::FilterOut);
}

#[test]
fn test_parser_nested_rules() {
    let mut tokens = scan_tokens("any(name = test, all(id = 123, geom.width > 400))");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 1);
    
    match &parsed[0].rule {
        wminspect::dsl::filter::FilterRule::Any(rules) => {
            assert_eq!(rules.len(), 2);
        }
        _ => panic!("Expected Any rule"),
    }
}

#[test]
fn test_parser_not_rule() {
    let mut tokens = scan_tokens("not(name = test)");
    let parsed = parse_rule(&mut tokens).unwrap();
    assert_eq!(parsed.len(), 1);
    
    match &parsed[0].rule {
        wminspect::dsl::filter::FilterRule::Not(_) => {
            // Success
        }
        _ => panic!("Expected Not rule"),
    }
}
