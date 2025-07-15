use wminspect::dsl::filter::{scan_tokens, Token, Op, Action};

#[test]
fn test_tokenizer_basic() {
    let tokens = scan_tokens("name = example");
    assert_eq!(tokens.len(), 4); // includes EOT
    assert_eq!(tokens[0], Token::StrLit("name".to_string()));
    assert_eq!(tokens[1], Token::OP(Op::Eq));
    assert_eq!(tokens[2], Token::StrLit("example".to_string()));
    assert_eq!(tokens[3], Token::EOT);
}

#[test]
fn test_tokenizer_operators() {
    let tokens = scan_tokens("id = 123");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0], Token::StrLit("id".to_string()));
    assert_eq!(tokens[1], Token::OP(Op::Eq));
    assert_eq!(tokens[2], Token::StrLit("123".to_string()));
    assert_eq!(tokens[3], Token::EOT);
}

#[test]
fn test_tokenizer_complex() {
    let tokens = scan_tokens("geom.width >= 400");
    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0], Token::StrLit("geom".to_string()));
    assert_eq!(tokens[1], Token::DOT);
    assert_eq!(tokens[2], Token::StrLit("width".to_string()));
    assert_eq!(tokens[3], Token::OP(Op::GE));
    assert_eq!(tokens[4], Token::StrLit("400".to_string()));
    assert_eq!(tokens[5], Token::EOT);
}

#[test]
fn test_tokenizer_keywords() {
    let tokens = scan_tokens("all(name = 'test')");
    assert_eq!(tokens.len(), 8);
    assert_eq!(tokens[0], Token::ALL);
    assert_eq!(tokens[1], Token::LBRACE);
    assert_eq!(tokens[2], Token::StrLit("name".to_string()));
    assert_eq!(tokens[3], Token::OP(Op::Eq));
    assert_eq!(tokens[4], Token::StrLit("test".to_string()));
    assert_eq!(tokens[5], Token::RBRACE);
    assert_eq!(tokens[6], Token::EOT);
}

#[test]
fn test_tokenizer_quoted_strings() {
    let tokens = scan_tokens("name = 'some complex string'");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0], Token::StrLit("name".to_string()));
    assert_eq!(tokens[1], Token::OP(Op::Eq));
    assert_eq!(tokens[2], Token::StrLit("some complex string".to_string()));
    assert_eq!(tokens[3], Token::EOT);
}

#[test]
fn test_tokenizer_actions() {
    let tokens = scan_tokens("name = test: pin; id = 123: filter");
    let mut found_pin = false;
    let mut found_filter = false;
    
    for token in tokens {
        match token {
            Token::ACTION(Action::Pin) => found_pin = true,
            Token::ACTION(Action::FilterOut) => found_filter = true,
            _ => {}
        }
    }
    
    assert!(found_pin);
    assert!(found_filter);
}
