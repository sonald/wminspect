extern crate serde;
extern crate serde_json;
extern crate bincode as bc;

use super::wm::*;
use std::collections::HashSet;
use std::convert::AsRef;

type FilterFunction = Box<Fn(&Window) -> bool + Send>;

pub struct ActionFuncPair {
    pub action: Action,
    pub(crate) rule: FilterRule,
    pub func: FilterFunction
}

pub struct Filter {
    pub rules: Vec<ActionFuncPair>
}

unsafe impl Sync for Filter {}

impl Filter {

    /// constructors
    pub fn new() -> Filter {
        Filter { rules: Vec::new(), }
    }

    pub fn parse<S: AsRef<str>>(rule: S) -> Filter {
        let mut filter = Filter { rules: Vec::new(), };

        let mut tokens = scan_tokens(rule);
        if let Some(top) = parse_rule(&mut tokens) {
            for item in top.into_iter() {
                wm_debug!("item: {:?}", item);
                let f = item.rule.gen_closure();
                filter.rules.push(ActionFuncPair { action: item.action, rule: item.rule, func: f});
            }
        }

        filter
    }

    pub fn apply_to(&self, w: &Window) -> bool {
        !self.rules.iter().filter(|r| r.action == Action::FilterOut).any(|r| !(r.func)(w))
    }

    pub fn add_live_rule(&mut self, item: ActionFuncPair) {
        self.rules.push(item);
    }
}


#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Action {
    FilterOut,
    Pin,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum Predicate {
    Id,
    Name,
    Attr(String), // String contains attr name (map_state or override_redirect)
    Geom(String), // String contains attr name (x,y,width,height)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) enum Matcher {
    IntegralValue(i16),
    BoolValue(bool),
    MapStateValue(MapState),
    Wildcard(String), // all string values are considered wildcard matcher
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum Op {
    Eq,
    Neq,
    GT,
    LT,
    GE,
    LE,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) enum FilterRule {
    Adhoc,
    ClientsOnly,
    Single { pred: Predicate, op: Op, matcher: Matcher },
    All (Vec<Box<FilterRule>>),
    Any (Vec<Box<FilterRule>>),
    Not (Box<FilterRule>)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct FilterItem {
    pub(crate) action: Action,
    pub(crate) rule: FilterRule,
}

type BoxedRule = Box<FilterRule>;

fn wild_match(pat: &str, s: &str) -> bool {
    // non recursive algorithm
    fn mat2(pat: &[char], s: &[char]) -> bool {
        let (mut i, mut j) = (0, 0);
        let mut star = usize::max_value();
        let mut k = 0;
        while j < s.len() {
            if pat.get(i).unwrap_or(&'\0') == &'?' || pat.get(i).unwrap_or(&'\0') == &s[j] {
                i += 1; j += 1; 
            } else if pat.get(i).unwrap_or(&'\0') == &'*' {
                star = i; k = j; i += 1; 
            } else if pat.get(star).is_some() {
                k += 1; j = k; i = star + 1;
            } else {
                return false;
            } 
        }

        while pat.get(i).unwrap_or(&'\0') == &'*' {
            i += 1; 
        }
        i == pat.len()
    }

    #[allow(dead_code)]
    fn mat_star(pat: &[char], i: usize, s: &[char], mut j: usize) -> bool {
        while j <= s.len() {
            if mat(pat, i+1, s, j) {
                return true;
            }
            j += 1;
        }

        false
    }

    #[allow(dead_code)]
    fn mat(pat: &[char], i: usize, s: &[char], j: usize) -> bool {
        if pat.len() == i || s.len() == j {
            return pat.len() == i && s.len() == j;
        }

        if pat[i] == '?' || pat[i] == s[j] {
            mat(pat, i+1, s, j+1)
        } else if pat[i] == '*' {
            mat_star(pat, i, s, j) 
        } else {
            return false;
        }
    }

    let res;
    if is_wild_string(pat) {
        //res = mat(&pat.chars().collect::<Vec<_>>(), 0, &s.chars().collect::<Vec<_>>(), 0);
        res = mat2(&pat.chars().collect::<Vec<_>>(), &s.chars().collect::<Vec<_>>());
    } else {
        res = s.contains(pat);
    }

    res
}

fn is_wild_string(pattern: &str) -> bool {
    pattern.chars().any(|c| c == '?' || c == '*')
}

fn parse_id(id_str: &str) -> u32 {
    let id_str = id_str.to_lowercase();
    if id_str.starts_with("0x") {
        u32::from_str_radix(&id_str[2..], 16).unwrap_or(0)
    } else {
        id_str.parse::<u32>().unwrap_or(0)
    }
} 

macro_rules! _match_geometry {
    ($elem:tt, $op:tt, $i:tt) => (
        match *$op {
            Op::Eq => Box::new(move |ref w| w.geom.$elem == $i),
            Op::Neq => Box::new(move |ref w| w.geom.$elem != $i),
            Op::GT => Box::new(move |ref w| w.geom.$elem > $i),
            Op::LT => Box::new(move |ref w| w.geom.$elem < $i),
            Op::GE => Box::new(move |ref w| w.geom.$elem >= $i),
            Op::LE => Box::new(move |ref w| w.geom.$elem <= $i),
        })
}

impl FilterRule {
    pub(crate) fn gen_closure(&self) -> FilterFunction {
        use self::FilterRule::*;
        match self {
            &Adhoc => {Box::new(|_w| true)},
            &ClientsOnly => FilterRule::clients_only_gen_closure(),
            &Single {ref pred, ref op, ref matcher} => FilterRule::single_gen_closure(pred, op, matcher),
            &All (ref rules) => FilterRule::all_gen_closure(rules),
            &Any (ref rules) => FilterRule::any_gen_closure(rules),
            &Not (ref rule) => FilterRule::not_gen_closure(rule),
        }
    }

    /// TODO: clients info can only be retreived from wm context
    fn clients_only_gen_closure() -> FilterFunction {
        Box::new(|_w|{true})
    }

    fn any_gen_closure(rules: &Vec<BoxedRule>) -> FilterFunction {
        let mut closures = Vec::new();
        for r in rules {
            closures.push(r.gen_closure())
        }

        Box::new(move |ref w| {
            for f in &closures {
                if f(w) {
                    return true;
                }
            }

            false
        })
    }

    fn not_gen_closure(rule: &BoxedRule) -> FilterFunction {
        let f = rule.gen_closure();
        Box::new(move |ref w| !f(w))
    }

    fn all_gen_closure(rules: &Vec<BoxedRule>) -> FilterFunction {
        let mut closures = Vec::new();
        for r in rules {
            closures.push(r.gen_closure())
        }

        Box::new(move |ref w| {
            for f in &closures {
                if !f(w) {
                    return false;
                }
            }

            true
        })
    }

    fn single_gen_closure(pred: &Predicate, op: &Op, matcher: &Matcher) -> FilterFunction {
        match (pred, op, matcher) {
            (&Predicate::Name, op, &Matcher::Wildcard(ref pat)) => {
                let pat = pat.clone();
                match *op {
                    Op::Eq => Box::new(move |ref w| wild_match(&pat, &w.name)),
                    Op::Neq => Box::new(move |ref w| !wild_match(&pat, &w.name)),
                    _ => {panic!("name can only use Eq|Neq as op")}
                }
                
            },
            (&Predicate::Id, &Op::Eq, &Matcher::Wildcard(ref id)) => {
                let id = id.clone();
                if is_wild_string(&id) {
                    Box::new(move |ref w| wild_match(&id, &format!("0x{:x}", w.id)))
                } else {
                    let i = parse_id(&id);
                    Box::new(move |ref w| w.id == i)
                }
            },
            (&Predicate::Attr(ref attr), op, &Matcher::MapStateValue(ref st)) if attr == "map_state" => {
                let state = *st;
                match *op {
                    Op::Eq => Box::new(move |ref w| w.attrs.map_state == state),
                    Op::Neq => Box::new(move |ref w| w.attrs.map_state != state),
                    _ => {panic!("map_state can only use Eq|Neq as op")}
                }
                
            },
            (&Predicate::Attr(ref attr), op, &Matcher::BoolValue(ref b)) if attr == "override_redirect" => {
                let or = *b;
                match *op {
                    Op::Eq => Box::new(move |ref w| w.attrs.override_redirect == or),
                    Op::Neq => Box::new(move |ref w| w.attrs.override_redirect != or),
                    _ => {panic!("override_redirect can only use Eq|Neq as op")}
                }
                
            },
            (&Predicate::Geom(ref g), op, &Matcher::IntegralValue(i)) => {
                match g.as_str() {
                    "x" => _match_geometry!(x, op, i),
                    "y" => _match_geometry!(y, op, i),
                    "width" => _match_geometry!(width, op, (i as u16)),
                    "height" => _match_geometry!(height, op, (i as u16)),
                    wrong @ _ => panic!("wrong geometry attribute {}", wrong)
                }
            },

            _ => {
                panic!("not implement"); 
            }
        }
    }
}




#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum Token {
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
pub(crate) type Tokens = VecDeque<Token>;

/// parse `Tokens` into FilterItem list
pub(crate) fn parse_rule(tokens: &mut Tokens) -> Option<Vec<FilterItem>> {
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

fn parse_cond(tokens: &mut Tokens) -> Option<FilterRule> {
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

                "clients" => {
                    return Some(FilterRule::ClientsOnly);
                },

                _ => { wm_debug!("wrong token"); }
            }

            assert!(tokens.len() >= 2);
            match (tokens.pop_front().unwrap(), tokens.pop_front().unwrap()) {
                (OP(ref op), StrLit(ref s)) => {
                    let matcher = match pred {
                        Predicate::Id => Matcher::Wildcard(s.clone()),
                        Predicate::Name => Matcher::Wildcard(s.clone()),
                        Predicate::Attr(ref a) if a == "override_redirect" => {
                            Matcher::BoolValue(match s.to_lowercase().as_str() {
                                "0" | "false" => false,
                                _ => true
                            })
                        },
                        Predicate::Attr(ref a) if a == "map_state" => {
                            Matcher::MapStateValue(match s.to_lowercase().as_str() {
                                "viewable" => MapState::Viewable,
                                "unmapped" => MapState::Unmapped,
                                "unviewable" => MapState::Unviewable,
                                _ => panic!("bad map state value")
                            })
                        },
                        Predicate::Attr(_) => panic!("bad attr name"),
                        Predicate::Geom(_) => Matcher::IntegralValue(s.parse::<i16>().unwrap_or(0))
                    };

                    Some(FilterRule::Single {
                        pred: pred,
                        op: op.clone(),
                        matcher: matcher
                    })
                }, 

                _ => {
                    wm_debug!("wrong rule");
                    None
                } 
            }
        },
        
        ANY | ALL => {
            match_tok!(tokens, LBRACE);
            let mut rules = Vec::new();
            while let Some(cond) = parse_cond(tokens) {
                rules.push(Box::new(cond));
                // pop ',' or ')' anyway
                let tk = tokens.pop_front().unwrap();
                if tk == RBRACE {
                    break
                }
            }

            if tk == ANY {
                Some(FilterRule::Any(rules))
            } else {
                Some(FilterRule::All(rules))
            }
        },

        NOT => {
            match_tok!(tokens, LBRACE);
            if let Some(cond) = parse_cond(tokens) {
                match_tok!(tokens, RBRACE);
                Some(FilterRule::Not(Box::new(cond))) //FIXME: assert only one rule included 
            } else {
                None
            }
        },
        _ => { wm_debug!("wrong match: [{:?}]", tk); None } 
    }
}

pub(crate) fn scan_tokens<S: AsRef<str>>(rule: S) -> Tokens {
    use self::Token::*;
    macro_rules! append_tok {
        ($tokens:tt, $tk:expr) => ({
            $tokens.push_back($tk); 
            //println!("collect [{:?}]", $tk);
        })
    }

    let mut tokens = Tokens::new();
    let mut chars = rule.as_ref().chars().peekable();
    let metas: HashSet<_> = ['.', ',', ';', ':', '(', ')', '<', '>', '='].iter().cloned().collect();
    let mut need_act = false;

    loop {
        let ch = match chars.next() {
            Some(c) => c,
            None => break,
        };

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
            ';' => { 
                append_tok!(tokens, SEMICOLON); 
                need_act = false;
            },
            ':' => { 
                append_tok!(tokens, COLON);
                need_act = true;
            },
            '(' => { append_tok!(tokens, LBRACE); },
            ')' => { append_tok!(tokens, RBRACE); },

            _ if ch.is_whitespace() => {},

            _ => {
                // scan string literal
                let compound_str = match ch {
                    '\'' | '"' => true,
                    _ => false
                };

                let mut s = String::new();
                if !compound_str { s.push(ch); }
                loop {
                    if compound_str {
                        match chars.peek() {
                            Some(&val) if val != '\'' && val != '"' => {},
                            _ => break,
                        }

                    } else {
                        match chars.peek() {
                            //skip special char
                            Some(val) if !metas.contains(val) => {},
                            _ => break,
                        }
                    }

                    s.push(chars.next().unwrap());
                }

                if compound_str {
                    chars.next(); // should be ' | "
                }

                s = s.trim().to_string();
                //wm_debug!("s = {}", s);

                match s.to_lowercase().as_str() {
                    "all" => append_tok!(tokens, ALL),
                    "any" => append_tok!(tokens, ANY),
                    "not" => append_tok!(tokens, NOT),
                    "pin" if need_act => append_tok!(tokens, ACTION(Action::Pin)),
                    "filter" if need_act => append_tok!(tokens, ACTION(Action::FilterOut)),
                    lowered @ _ => append_tok!(tokens, StrLit(lowered.to_string()))
                }
            }
        } 
    }

    append_tok!(tokens, EOT);
    tokens
}

pub fn filter_grammar() ->&'static str {
    return "grammar:
    top -> ( item ( ';' item )* )?
    item -> cond ( ':' action)? 
        | 'clients'
    cond -> pred op VAL
        | ANY '(' cond (',' cond )* ')'
        | ALL '(' cond (',' cond )* ')'
        | NOT '(' cond ')'
        | 'clients'
    pred -> ID ('.' ID)*
    op -> '=' | '>' | '<' | '>=' | '<=' | '<>'
    action -> 'filter' | 'pin'
    ID -> STRING_LIT
    VAL -> STRING_LIT
    
pred could be:
    attrs.(map_state|override_redirect)
    geom.(x|y|width|height)
    id
    name
";
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
    fn test_scan_tokens5() {
        // compound string literal
        let tokens = scan_tokens("any(name = 'inside, this', name = \"name ; any\"): pin".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 13);
    }

    #[test]
    fn test_parse_flow() {
        let mut tokens = scan_tokens("any(name =dde?osd*, all(geom.x > 2, geom.width < 500));".to_string());
        println!("tokens: {:?}", tokens);
        assert_eq!(tokens.len(), 23);

        let rule = parse_rule(&mut tokens);
        println!("rule: {:?}", rule);
        assert!(rule.is_some());
        assert_eq!(rule.unwrap().len(), 1);
    }

    #[test]
    fn test_wild_match() {
        assert!(is_wild_string("dde*"));
        assert!(is_wild_string("*"));
        assert!(is_wild_string("dde?desktop"));
        assert!(!is_wild_string("dde-desktop"));

        assert_eq!(wild_match("dd?", "dde"), true);
        assert_eq!(wild_match("dd*", "dde"), true);
        assert_eq!(wild_match("dde*", "dde-osd"), true);
        assert_eq!(wild_match("dde*osd", "dde-osd"), true);
        assert_eq!(wild_match("dde*-osd", "dde-osd"), true);
        assert_eq!(wild_match("dd*?osd", "dde-osd"), true);
        assert_eq!(wild_match("dd*?", "dde-osd"), true);
        assert_eq!(wild_match("dd?*", "dde-osd"), true);
        assert_eq!(wild_match("*dde*", "dde-osd"), true);
        assert_eq!(wild_match("*dde*", "dde-desktop"), true);
        assert_eq!(wild_match("*?", "dde-osd"), true);
        assert_eq!(wild_match("?*", "dde-osd"), true);
        assert_eq!(wild_match("*", "dde-osd"), true);
        assert_eq!(wild_match("?*d", "dde-osd"), true);
        assert_eq!(wild_match("*d", "dde-osd"), true);
        assert_eq!(wild_match("???*sd", "dde-osd"), true);
        assert_eq!(wild_match("??*-*", "deepin-wm-switcher"), true);
        assert_eq!(wild_match("??*-*-??", "deepin-wm-switcher"), false);
        assert_eq!(wild_match("??*-wm-*", "deepin-wm-switcher"), true);

        assert_eq!(wild_match("*dde*", "ClutterActor: Clutter Reference Manual"), false);
    }

    #[test]
    fn test_whole() {
        use super::super::sheets::SheetFormat;
        let mut filter = Filter::new();
        filter.extend_with("name = dde*;".to_string(), SheetFormat::Plain);
        assert_eq!(filter.rules.len(), 1);
    }

    #[test]
    fn test_store1() {
        let act = Action::FilterOut; 
        let serialized = serde_json::to_string(&act).unwrap();
        println!("serialized = {}", serialized);
        let act2 = serde_json::from_str::<Action>(&serialized).unwrap();
        assert_eq!(act, act2);


        let act = Action::Pin; 
        let serialized = serde_json::to_string(&act).unwrap();
        println!("serialized = {}", serialized);
        let act2 = serde_json::from_str::<Action>(&serialized).unwrap();
        assert_eq!(act, act2);

        let mut tokens = scan_tokens("all(name = dde*, geom.x > 100);".to_string());
        if let Some(top) = parse_rule(&mut tokens) {
            let serialized = serde_json::to_string(&top).unwrap();
            println!("serialized = {}", serialized);
        }
    }

    #[test]
    fn test_store2() {
        let mut tokens = scan_tokens("name = dde*;".to_string());
        if let Some(top) = parse_rule(&mut tokens) {
            let serialized = serde_json::to_string(&top).unwrap();
            println!("serialized = {}", serialized);

            //let json = &mut serde_json::de::Deserializer::from_slice(serialized.as_bytes());
            //let mut format = Deserializer::erase(json);
            //let top2: Vec<FilterItem> = deserialize(&mut format).unwrap();

            let act2 = serde_json::from_str::<Vec<FilterItem>>(&serialized).unwrap();
            println!("deserialized = {:?}", &act2);
            assert_eq!(top, act2);
        }
    }

    #[test]
    fn test_store3() {
        let r = r#"
        any(all(geom.x > 0, geom.y > 0), 
            all(name = dde*, geom.x > 100)): filter;
        not(attr.map_state == unmapped)
        "#;
        let mut tokens = scan_tokens(r.to_string());
        if let Some(top) = parse_rule(&mut tokens) {
            let serialized = serde_json::to_string(&top).unwrap();
            println!("serialized = {}", serialized);

            let act2 = serde_json::from_str::<Vec<FilterItem>>(&serialized).unwrap();
            println!("deserialized = {:?}", &act2);
            assert_eq!(top, act2);
        }
    }

}
