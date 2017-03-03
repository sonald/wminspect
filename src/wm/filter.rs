extern crate regex;

use super::wm::*;
use std::fmt::Debug;
use self::regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone)]
enum Condition {
    Colorful,
    MappedOnly,
    OmitHidden,
    NoSpecial,
}

type FilterFunction = Box<Fn(&Window) -> bool + Send>;

pub struct Filter {
    options: Vec<Condition>,
    pub applys: Vec<FilterFunction>
}

unsafe impl Sync for Filter {}

macro_rules! build_fun {
    ($getter:ident, $setter:ident, $cond:tt) => (
        pub fn $getter(&self) -> bool {
            self.options.as_slice().iter().any(|c| {
                match *c {
                    Condition::$cond => true,
                    _ => false
                }
            })
        }
        
        pub fn $setter(&mut self) {
            self.options.push(Condition::$cond)
        })
}

impl Filter {
    build_fun!(mapped_only, set_mapped_only, MappedOnly);
    build_fun!(colorful, set_colorful, Colorful);
    build_fun!(omit_hidden, set_omit_hidden, OmitHidden);
    build_fun!(no_special, set_no_special, NoSpecial);
}


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Action {
    FilterOut,
    Pin,
}

#[derive(Debug, Clone)]
enum Predicate {
    Id,
    Name,
    Attr(String), // String contains attr name (map_state or override_redirect)
    Geom(String), // String contains attr name (x,y,width,height)
}

#[derive(Debug, Clone)]
enum Matcher {
    IntegralValue(i32),
    BoolValue(bool),
    MapStateValue(MapState),
    Wildcard(String), // all string values are considered wildcard matcher
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
enum Op {
    Eq,
    Neq,
    GT,
    LT,
    GE,
    LE,
}

// marker
trait Rule : Debug { }

type BoxedRule = Box<Rule + Send>;

#[derive(Debug)]
struct FilterItem {
    action: Action,
    rule: BoxedRule,
}


#[derive(Debug)]
struct FilterRule {
    pred: Predicate,
    op: Op,
    matcher: Matcher
}

#[derive(Debug)]
struct All {
    rules: Vec<BoxedRule>
}

#[derive(Debug)]
struct Any {
    rules: Vec<BoxedRule>
}

#[derive(Debug)]
struct Not {
    rule: BoxedRule
}

impl Rule for FilterRule { }
impl Rule for All { }
impl Rule for Any { }
impl Rule for Not { }




#[derive(Debug, PartialEq, Eq, Clone)]
enum Token {
    OP(Op),
    StrLit(String), // ID or VALUE
    ACTION(Action),
    ANY,
    ALL,
    NOT,
    LBRACE,
    RBRACE,
    COMMA,
    COLON,
    SEMICOLON,
    DOT,
    EOT, // special
}

use std::collections::VecDeque;
type Tokens = VecDeque<Token>;

/// grammar:
/// top -> ( item ( ';' item )* )?
/// item -> cond ( ':' action)? 
/// cond -> pred op VAL
///     | ANY '(' cond (',' cond )* ')'
///     | ALL '(' cond (',' cond )* ')'
///     | NOT '(' cond ')'
/// pred -> ID ('.' ID)*
/// op -> '=' | '>' | '<' | '>=' | '<=' | '<>'
/// action -> 'filter' | 'pin'
/// ID -> STRING_LIT
/// VAL -> STRING_LIT
fn parse_rule(tokens: &mut Tokens) -> Option<Vec<FilterItem>> {
    use self::Token::*;

    let mut items = Vec::new();
    while let Some(item) = parse_item(tokens) {
        items.push(item);
        let tk = tokens.pop_front().unwrap();
        if tk == EOT {
            break;
        }
    }

    Some(items)
}

fn parse_item(tokens: &mut Tokens) -> Option<FilterItem> {
    use self::Token::*;

    let mut action = Action::FilterOut;

    if tokens[0] == EOT { 
        return None;
    }

    match parse_cond(tokens) {
        Some(cond) => {
            if tokens[0] == COLON {
                tokens.pop_front();
                match tokens.pop_front().unwrap() {
                    ACTION(act) => action = act,
                    _ => {wm_debug!("ignore wrong action")}
                }
            }

            Some(FilterItem {action: action, rule: cond })
        }, 
        _ => {
            return None
        }
    }
}

macro_rules! match_tok {
    ($tokens:tt, $kd:expr) => (
        {
            if $tokens[0] == $kd {
                $tokens.pop_front().unwrap();
            } else {
                panic!("expecting {:?} but {:?}", $kd, $tokens[0]);
            }
        }
    )
}

fn parse_cond(tokens: &mut Tokens) -> Option<BoxedRule> {
    use self::Token::*;

    let tk = tokens.pop_front().unwrap();
    match tk {
        StrLit(ref s) => {
            let mut pred = Predicate::Id;

            match s.as_str() {
                "attrs" => { 
                    match_tok!(tokens, DOT);
                    let tk = tokens.pop_front().unwrap();
                    if let StrLit(name) = tk {
                        assert!(name == "map_state" || name == "override_redirect");
                        pred = Predicate::Attr(name);
                    } else {
                        wm_debug!("wrong token");
                    }
                },
                "geom" => {
                    match_tok!(tokens, DOT);
                    let tk = tokens.pop_front().unwrap();
                    if let StrLit(name) = tk {
                        assert!(name == "x" || name == "y" || name == "width" || name == "height");
                        pred = Predicate::Geom(name);
                    } else {
                        wm_debug!("wrong token");
                    }
                },

                "id" | "name" => {
                    pred = if s == "id" { Predicate::Id } else { Predicate::Name };
                },

                _ => { wm_debug!("wrong token"); }
            }

            assert!(tokens.len() >= 2);
            match (tokens.pop_front().unwrap(), tokens.pop_front().unwrap()) {
                (OP(ref op), StrLit(ref s)) => {
                    Some(Box::new(FilterRule {
                        pred: pred,
                        op: op.clone(),
                        matcher: Matcher::Wildcard(s.clone())
                    }))
                }, 

                _ => {
                    wm_debug!("wrong rule");
                    None
                } 
            }
        },
        
        ANY | ALL | NOT => {
            match_tok!(tokens, LBRACE);
            let mut rules = Vec::new();
            while let Some(cond) = parse_cond(tokens) {
                rules.push(cond);
                // pop ',' or ')' anyway
                let tk = tokens.pop_front().unwrap();
                if tk == RBRACE {
                    break
                }
            }

            if tk == ANY {
                Some(Box::new(Any {rules: rules}))
            } else {
                Some(Box::new(All {rules: rules}))
            }
        },

        NOT => {
            match_tok!(tokens, LBRACE);
            if let Some(cond) = parse_cond(tokens) {
                match_tok!(tokens, RBRACE);
                Some(Box::new(Not {rule: cond})) //FIXME: assert only one rule included 
            } else {
                None
            }
        },
        _ => { wm_debug!("wrong match"); None } 
    }
}

fn scan_tokens(rule: String) -> Tokens {
    use self::Token::*;
    macro_rules! append_tok {
        ($tokens:tt, $tk:expr) => ({
            $tokens.push_back($tk); 
            println!("collect [{:?}]", $tk);
        })
    }

    let mut tokens = Tokens::new();

    let mut chars = rule.chars().peekable();

    let metas: HashSet<_> = ['.', ',', ';', ':', '(', ')', '<', '>', '='].iter().cloned().collect();

    loop {
        let ch = match chars.next() {
            Some(c) => c,
            None => break,
        };

        wm_debug!("ch = {}", ch); 
        match ch {
            '=' => {
                append_tok!(tokens, OP(Op::Eq));
            },
            
            '>' => {
                let mut do_consume = false;
                if let Some(nt) = chars.peek() {
                    if *nt == '=' {
                        append_tok!(tokens, OP(Op::GE));
                        do_consume = true
                    } else {
                        append_tok!(tokens, OP(Op::GT));
                    }
                }

                if do_consume { chars.next(); }
            },

            '<' => {
                let mut do_consume = false;
                if let Some(nt) = chars.peek() {
                    if *nt == '=' {
                        append_tok!(tokens, OP(Op::LE));
                        do_consume = true
                    } else if *nt == '>' {
                        append_tok!(tokens, OP(Op::Neq));
                        do_consume = true
                    } else {
                        append_tok!(tokens, OP(Op::LT));
                    }
                }
                if do_consume { chars.next(); }
            },

            '.' => { append_tok!(tokens, DOT); },
            ',' => { append_tok!(tokens, COMMA); },
            ';' => { append_tok!(tokens, SEMICOLON); },
            ':' => { append_tok!(tokens, COLON); },
            '(' => { append_tok!(tokens, LBRACE); },
            ')' => { append_tok!(tokens, RBRACE); },

            _ if ch.is_whitespace() => {},

            _ => {
                // scan string literal
                let mut s = String::new();
                s.push(ch);
                loop {
                    {
                        match chars.peek() {
                            //skip special char
                            Some(val) if !metas.contains(val) => {},
                            _ => break,
                        }
                    }

                    s.push(chars.next().unwrap());
                }

                s = s.trim().to_string();
                wm_debug!("s = {}", s);

                match s.to_lowercase().as_str() {
                    "all" => append_tok!(tokens, ALL),
                    "any" => append_tok!(tokens, ANY),
                    "not" => append_tok!(tokens, NOT),
                    lowered @ _ => append_tok!(tokens, StrLit(lowered.to_string()))
                }
            }
        } 
    }

    append_tok!(tokens, EOT);
    tokens
}

pub fn parse_filter(rule: String) -> Filter {
    let mut filter = Filter {options: Vec::new(), applys: Vec::new()};
    
    let mut tokens = scan_tokens(rule);
    let top = parse_rule(&mut tokens);
    println!("{:?}", top);

    filter.applys.push(Box::new(move |w: &Window| w.name.contains("mutter")));

    filter
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::Token::*;

    macro_rules! append_tok {
        ($tokens:tt, $tk:expr) => ( $tokens.push_back($tk); )
    }

    #[test]
    fn test_parse_rule() {
        let mut tokens = Tokens::new();
        append_tok!(tokens, StrLit("name".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("dde-osd".to_string()));
        append_tok!(tokens, EOT);

        let rule = parse_rule(&mut tokens);
        println!("{:?}", rule);
        assert!(rule.is_some());
    }

    #[test]
    fn test_parse_rule2() {
        let mut tokens = Tokens::new();
        append_tok!(tokens, StrLit("name".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("dde-osd".to_string()));
        append_tok!(tokens, COLON);
        append_tok!(tokens, ACTION(Action::Pin));

        append_tok!(tokens, COMMA);

        append_tok!(tokens, StrLit("id".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("0x8a000??".to_string()));
        append_tok!(tokens, EOT);

        let rule = parse_rule(&mut tokens);
        println!("{:?}", rule);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_rule3() {
        let mut tokens = Tokens::new();
        append_tok!(tokens, StrLit("attrs".to_string()));
        append_tok!(tokens, DOT);
        append_tok!(tokens, StrLit("map_state".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("Viewable".to_string()));
        append_tok!(tokens, COLON);
        append_tok!(tokens, ACTION(Action::Pin));

        append_tok!(tokens, COMMA);

        append_tok!(tokens, StrLit("id".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("0x8a000??".to_string()));
        append_tok!(tokens, EOT);

        let rule = parse_rule(&mut tokens);
        println!("{:?}", rule);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_rule4() {
        let mut tokens = Tokens::new();
        append_tok!(tokens, ALL);
        append_tok!(tokens, LBRACE);
        append_tok!(tokens, StrLit("attrs".to_string()));
        append_tok!(tokens, DOT);
        append_tok!(tokens, StrLit("map_state".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("Viewable".to_string()));
        append_tok!(tokens, COMMA);
        append_tok!(tokens, StrLit("geom".to_string()));
        append_tok!(tokens, DOT);
        append_tok!(tokens, StrLit("width".to_string()));
        append_tok!(tokens, OP(Op::GE));
        append_tok!(tokens, StrLit("400".to_string()));
        append_tok!(tokens, RBRACE);
        append_tok!(tokens, COLON);
        append_tok!(tokens, ACTION(Action::Pin));

        append_tok!(tokens, COMMA);

        append_tok!(tokens, StrLit("id".to_string()));
        append_tok!(tokens, OP(Op::Eq));
        append_tok!(tokens, StrLit("0x8a000??".to_string()));
        append_tok!(tokens, EOT);

        let rule = parse_rule(&mut tokens);
        println!("{:?}", rule);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().len(), 2);
    }

    #[test]
    fn test_scan_tokens1() {
        let tokens = scan_tokens("not(attrs.map_state = Viewable): pin;".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 12);
    }

    #[test]
    fn test_scan_tokens2() {
        let tokens = scan_tokens("not(geom.x > 2, geom.width < 500): filter;".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 18);
    }

    #[test]
    fn test_scan_tokens3() {
        let tokens = scan_tokens("any(name =dde?osd, all(geom.x > 2, geom.width < 500));".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 23);
    }

    #[test]
    fn test_scan_tokens4() {
        let tokens = scan_tokens("not(name =dde?osd): pin; attrs.map_state=Viewable;".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 16);
    }

    #[test]
    fn test_parse_flow() {
        let mut tokens = scan_tokens("any(name =dde?osd*, all(geom.x > 2, geom.width < 500));".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 23);

        let rule = parse_rule(&mut tokens);
        println!("{:?}", rule);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().len(), 1);
    }
}
