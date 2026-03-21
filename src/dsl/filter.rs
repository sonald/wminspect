extern crate bincode as bc;
extern crate serde;
extern crate serde_json;

use crate::core::types::*;
use crate::core::wildcard::OptimizedWildcardMatcher;
use crate::{wm_error, wm_trace};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::AsRef;

type FilterFunction = Box<dyn Fn(&Window) -> bool + Send>;

pub struct ActionFuncPair {
    pub action: Action,
    pub(crate) rule: FilterRule,
    pub func: FilterFunction,
}

impl std::fmt::Debug for ActionFuncPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActionFuncPair")
            .field("action", &self.action)
            .field("rule", &self.rule)
            .field("func", &"<function>")
            .finish()
    }
}

#[derive(Debug)]
pub struct Filter {
    pub rules: Vec<ActionFuncPair>,
}

unsafe impl Sync for Filter {}

impl Filter {
    /// constructors
    pub fn new() -> Filter {
        Filter { rules: Vec::new() }
    }

    pub fn parse<S: AsRef<str>>(rule: S) -> Filter {
        let mut filter = Filter { rules: Vec::new() };

        let mut tokens = scan_tokens(rule);
        if let Some(top) = parse_rule(&mut tokens) {
            for item in top.into_iter() {
                wm_trace!("item: {:?}", item);
                let f = item.rule.gen_closure();
                filter.rules.push(ActionFuncPair {
                    action: item.action,
                    rule: item.rule,
                    func: f,
                });
            }
        }

        filter
    }

    pub fn apply_to(&self, w: &Window) -> bool {
        !self
            .rules
            .iter()
            .filter(|r| r.action == Action::FilterOut)
            .any(|r| !(r.func)(w))
    }

    pub fn add_live_rule(&mut self, item: ActionFuncPair) {
        self.rules.push(item);
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Replace all rules with new ones
    pub fn replace_rules(&mut self, new_rules: Vec<ActionFuncPair>) {
        self.rules = new_rules;
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get pinned window count from rules
    pub fn pinned_rule_count(&self) -> usize {
        self.rules
            .iter()
            .filter(|r| r.action == Action::Pin)
            .count()
    }

    /// Get filter rule count
    pub fn filter_rule_count(&self) -> usize {
        self.rules
            .iter()
            .filter(|r| r.action == Action::FilterOut)
            .count()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Action {
    FilterOut,
    Pin,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    Id,
    Name,
    Attr(String), // String contains attr name (map_state or override_redirect)
    Geom(String), // String contains attr name (x,y,width,height)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    IntegralValue(i16),
    BoolValue(bool),
    MapStateValue(MapState),
    Wildcard(String), // all string values are considered wildcard matcher
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Op {
    Eq,
    Neq,
    GT,
    LT,
    GE,
    LE,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum FilterRule {
    Adhoc,
    ClientsOnly,
    Single {
        pred: Predicate,
        op: Op,
        matcher: Matcher,
    },
    All(Vec<Box<FilterRule>>),
    Any(Vec<Box<FilterRule>>),
    Not(Box<FilterRule>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct FilterItem {
    pub action: Action,
    pub rule: FilterRule,
}

type BoxedRule = Box<FilterRule>;

pub fn wild_match(pat: &str, s: &str) -> bool {
    // non recursive algorithm
    fn mat2(pat: &[char], s: &[char]) -> bool {
        let (mut i, mut j) = (0, 0);
        let mut star = usize::max_value();
        let mut k = 0;
        while j < s.len() {
            if pat.get(i).unwrap_or(&'\0') == &'?' || pat.get(i).unwrap_or(&'\0') == &s[j] {
                i += 1;
                j += 1;
            } else if pat.get(i).unwrap_or(&'\0') == &'*' {
                star = i;
                k = j;
                i += 1;
            } else if pat.get(star).is_some() {
                k += 1;
                j = k;
                i = star + 1;
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
            if mat(pat, i + 1, s, j) {
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
            mat(pat, i + 1, s, j + 1)
        } else if pat[i] == '*' {
            mat_star(pat, i, s, j)
        } else {
            return false;
        }
    }

    let res;
    if is_wild_string(pat) {
        //res = mat(&pat.chars().collect::<Vec<_>>(), 0, &s.chars().collect::<Vec<_>>(), 0);
        res = mat2(
            &pat.chars().collect::<Vec<_>>(),
            &s.chars().collect::<Vec<_>>(),
        );
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
    ($elem:tt, $op:tt, $i:tt) => {
        match *$op {
            Op::Eq => Box::new(move |ref w| w.geom.$elem == $i),
            Op::Neq => Box::new(move |ref w| w.geom.$elem != $i),
            Op::GT => Box::new(move |ref w| w.geom.$elem > $i),
            Op::LT => Box::new(move |ref w| w.geom.$elem < $i),
            Op::GE => Box::new(move |ref w| w.geom.$elem >= $i),
            Op::LE => Box::new(move |ref w| w.geom.$elem <= $i),
        }
    };
}

impl FilterRule {
    pub(crate) fn gen_closure(&self) -> FilterFunction {
        use self::FilterRule::*;
        match self {
            &Adhoc => Box::new(|_w| true),
            &ClientsOnly => FilterRule::clients_only_gen_closure(),
            &Single {
                ref pred,
                ref op,
                ref matcher,
            } => FilterRule::single_gen_closure(pred, op, matcher),
            &All(ref rules) => FilterRule::all_gen_closure(rules),
            &Any(ref rules) => FilterRule::any_gen_closure(rules),
            &Not(ref rule) => FilterRule::not_gen_closure(rule),
        }
    }

    /// TODO: clients info can only be retreived from wm context
    fn clients_only_gen_closure() -> FilterFunction {
        Box::new(|w| !w.attrs.override_redirect && w.attrs.map_state != MapState::Unmapped)
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
                    Op::Eq => Box::new(move |ref w| {
                        OptimizedWildcardMatcher::match_pattern(&pat, &w.name)
                    }),
                    Op::Neq => Box::new(move |ref w| {
                        !OptimizedWildcardMatcher::match_pattern(&pat, &w.name)
                    }),
                    _ => {
                        panic!("name can only use Eq|Neq as op")
                    }
                }
            }
            (&Predicate::Id, &Op::Eq, &Matcher::Wildcard(ref id)) => {
                let id = id.clone();
                if is_wild_string(&id) {
                    Box::new(move |ref w| {
                        OptimizedWildcardMatcher::match_pattern(&id, &format!("0x{:x}", w.id))
                    })
                } else {
                    let i = parse_id(&id);
                    Box::new(move |ref w| w.id == i)
                }
            }
            (&Predicate::Attr(ref attr), op, &Matcher::MapStateValue(ref st))
                if attr == "map_state" =>
            {
                let state = *st;
                match *op {
                    Op::Eq => Box::new(move |ref w| w.attrs.map_state == state),
                    Op::Neq => Box::new(move |ref w| w.attrs.map_state != state),
                    _ => {
                        panic!("map_state can only use Eq|Neq as op")
                    }
                }
            }
            (&Predicate::Attr(ref attr), op, &Matcher::BoolValue(ref b))
                if attr == "override_redirect" =>
            {
                let or = *b;
                match *op {
                    Op::Eq => Box::new(move |ref w| w.attrs.override_redirect == or),
                    Op::Neq => Box::new(move |ref w| w.attrs.override_redirect != or),
                    _ => {
                        panic!("override_redirect can only use Eq|Neq as op")
                    }
                }
            }
            (Predicate::Geom(g), op, &Matcher::IntegralValue(i)) => match g.as_str() {
                "x" => _match_geometry!(x, op, i),
                "y" => _match_geometry!(y, op, i),
                "width" => _match_geometry!(width, op, (i as u16)),
                "height" => _match_geometry!(height, op, (i as u16)),
                wrong => panic!("wrong geometry attribute {wrong}"),
            },

            // Handle Id with IntegralValue matcher
            (&Predicate::Id, op, &Matcher::IntegralValue(i)) => match *op {
                Op::Eq => Box::new(move |ref w| w.id == (i as u32)),
                Op::Neq => Box::new(move |ref w| w.id != (i as u32)),
                Op::GT => Box::new(move |ref w| w.id > (i as u32)),
                Op::LT => Box::new(move |ref w| w.id < (i as u32)),
                Op::GE => Box::new(move |ref w| w.id >= (i as u32)),
                Op::LE => Box::new(move |ref w| w.id <= (i as u32)),
            },

            // Handle Name with IntegralValue matcher (convert to string comparison)
            (&Predicate::Name, op, &Matcher::IntegralValue(i)) => {
                let s = i.to_string();
                match *op {
                    Op::Eq => Box::new(move |ref w| w.name == s),
                    Op::Neq => Box::new(move |ref w| w.name != s),
                    _ => Box::new(|_w| false), // Other ops don't make sense for names
                }
            }

            // Handle Geom with Wildcard matcher (convert to string comparison)
            (&Predicate::Geom(ref g), op, &Matcher::Wildcard(ref pat)) => {
                let pat = pat.clone();
                let geo_attr = g.clone();
                match *op {
                    Op::Eq => Box::new(move |ref w| {
                        let value_str = match geo_attr.as_str() {
                            "x" => w.geom.x.to_string(),
                            "y" => w.geom.y.to_string(),
                            "width" => w.geom.width.to_string(),
                            "height" => w.geom.height.to_string(),
                            _ => return false,
                        };
                        OptimizedWildcardMatcher::match_pattern(&pat, &value_str)
                    }),
                    Op::Neq => Box::new(move |ref w| {
                        let value_str = match geo_attr.as_str() {
                            "x" => w.geom.x.to_string(),
                            "y" => w.geom.y.to_string(),
                            "width" => w.geom.width.to_string(),
                            "height" => w.geom.height.to_string(),
                            _ => return false,
                        };
                        !OptimizedWildcardMatcher::match_pattern(&pat, &value_str)
                    }),
                    _ => Box::new(|_w| false), // Other ops with wildcards don't make sense
                }
            }

            // Fallback case - log the unimplemented combination
            (pred, op, matcher) => {
                wm_error!(
                    "Unimplemented predicate/op/matcher combination: {:?}/{:?}/{:?}",
                    pred,
                    op,
                    matcher
                );
                Box::new(|_w| false)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
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
pub type ParseDiagnostics = Vec<String>;

fn push_diag<S: Into<String>>(diagnostics: &mut ParseDiagnostics, message: S) {
    diagnostics.push(message.into());
}

fn pop_token(
    tokens: &mut Tokens,
    diagnostics: &mut ParseDiagnostics,
    expected: &str,
) -> Option<Token> {
    match tokens.pop_front() {
        Some(token) => Some(token),
        None => {
            push_diag(
                diagnostics,
                format!("expected {expected}, found end of input"),
            );
            None
        }
    }
}

fn peek_token(tokens: &Tokens) -> Option<&Token> {
    tokens.front()
}

fn expect_token(tokens: &mut Tokens, expected: Token, diagnostics: &mut ParseDiagnostics) -> bool {
    match tokens.front() {
        Some(token) if *token == expected => {
            tokens.pop_front();
            true
        }
        Some(token) => {
            push_diag(
                diagnostics,
                format!("expected {:?}, found {:?}", expected, token),
            );
            false
        }
        None => {
            push_diag(
                diagnostics,
                format!("expected {:?}, found end of input", expected),
            );
            false
        }
    }
}

fn parse_bool_value(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_map_state_value(value: &str) -> Option<MapState> {
    match value.to_ascii_lowercase().as_str() {
        "viewable" => Some(MapState::Viewable),
        "unmapped" => Some(MapState::Unmapped),
        "unviewable" => Some(MapState::Unviewable),
        _ => None,
    }
}

fn parse_matcher(
    pred: &Predicate,
    raw_value: &str,
    diagnostics: &mut ParseDiagnostics,
) -> Option<Matcher> {
    match pred {
        Predicate::Id => Some(Matcher::Wildcard(raw_value.to_string())),
        Predicate::Name => Some(Matcher::Wildcard(raw_value.to_string())),
        Predicate::Attr(attr) if attr == "override_redirect" => parse_bool_value(raw_value)
            .map(Matcher::BoolValue)
            .or_else(|| {
                push_diag(
                    diagnostics,
                    format!(
                        "invalid override_redirect value {:?}; expected true, false, 1, or 0",
                        raw_value
                    ),
                );
                None
            }),
        Predicate::Attr(attr) if attr == "map_state" => parse_map_state_value(raw_value)
            .map(Matcher::MapStateValue)
            .or_else(|| {
                push_diag(
                    diagnostics,
                    format!(
                        "invalid map_state value {:?}; expected viewable, unmapped, or unviewable",
                        raw_value
                    ),
                );
                None
            }),
        Predicate::Attr(attr) => {
            push_diag(
                diagnostics,
                format!("unsupported attribute predicate {:?}", attr),
            );
            None
        }
        Predicate::Geom(_) => raw_value
            .parse::<i16>()
            .map(Matcher::IntegralValue)
            .ok()
            .or_else(|| {
                push_diag(
                    diagnostics,
                    format!("invalid geometry value {:?}; expected integer", raw_value),
                );
                None
            }),
    }
}

/// parse `Tokens` into FilterItem list
pub fn parse_rule(tokens: &mut Tokens) -> Option<Vec<FilterItem>> {
    match parse_rule_with_diagnostics(tokens) {
        Ok(items) => Some(items),
        Err(errors) => {
            for error in errors {
                wm_error!("{}", error);
            }
            None
        }
    }
}

pub fn parse_rule_with_diagnostics(
    tokens: &mut Tokens,
) -> Result<Vec<FilterItem>, ParseDiagnostics> {
    use self::Token::*;

    let mut diagnostics = Vec::new();
    let mut items = Vec::new();

    loop {
        match peek_token(tokens) {
            Some(EOT) => {
                tokens.pop_front();
                break;
            }
            None => break,
            _ => {}
        }

        let Some(item) = parse_item(tokens, &mut diagnostics) else {
            return Err(diagnostics);
        };
        items.push(item);

        match peek_token(tokens) {
            Some(SEMICOLON) | Some(COMMA) => {
                tokens.pop_front();
            }
            Some(EOT) => {
                tokens.pop_front();
                break;
            }
            Some(token) => {
                push_diag(
                    &mut diagnostics,
                    format!("expected rule separator or end of input, found {:?}", token),
                );
                return Err(diagnostics);
            }
            None => break,
        }
    }

    Ok(items)
}

pub fn parse_rule_text_with_diagnostics<S: AsRef<str>>(
    rule: S,
) -> Result<Vec<FilterItem>, ParseDiagnostics> {
    let mut tokens = scan_tokens(rule);
    parse_rule_with_diagnostics(&mut tokens)
}

fn parse_item(tokens: &mut Tokens, diagnostics: &mut ParseDiagnostics) -> Option<FilterItem> {
    use self::Token::*;

    let mut action = Action::FilterOut;

    if matches!(peek_token(tokens), Some(EOT) | None) {
        return None;
    }

    let cond = parse_cond(tokens, diagnostics)?;

    if matches!(peek_token(tokens), Some(COLON)) {
        tokens.pop_front();
        match pop_token(tokens, diagnostics, "action")? {
            ACTION(act) => action = act,
            token => {
                push_diag(
                    diagnostics,
                    format!("expected action after ':', found {:?}", token),
                );
                return None;
            }
        }
    }

    Some(FilterItem { action, rule: cond })
}

fn parse_predicate(
    first: String,
    tokens: &mut Tokens,
    diagnostics: &mut ParseDiagnostics,
) -> Option<Predicate> {
    use self::Token::*;

    match first.as_str() {
        "attrs" => {
            if !expect_token(tokens, DOT, diagnostics) {
                return None;
            }
            match pop_token(tokens, diagnostics, "attribute name")? {
                StrLit(name) if name == "map_state" || name == "override_redirect" => {
                    Some(Predicate::Attr(name))
                }
                StrLit(name) => {
                    push_diag(diagnostics, format!("wrong attr token: {:?}", name));
                    None
                }
                token => {
                    push_diag(diagnostics, format!("wrong token: {:?}", token));
                    None
                }
            }
        }
        "geom" => {
            if !expect_token(tokens, DOT, diagnostics) {
                return None;
            }
            match pop_token(tokens, diagnostics, "geometry field")? {
                StrLit(name)
                    if name == "x" || name == "y" || name == "width" || name == "height" =>
                {
                    Some(Predicate::Geom(name))
                }
                StrLit(name) => {
                    push_diag(diagnostics, format!("wrong geometry token: {:?}", name));
                    None
                }
                token => {
                    push_diag(diagnostics, format!("wrong token: {:?}", token));
                    None
                }
            }
        }
        "id" => Some(Predicate::Id),
        "name" => Some(Predicate::Name),
        other => {
            push_diag(diagnostics, format!("wrong token: {:?}", other));
            None
        }
    }
}

fn parse_cond(tokens: &mut Tokens, diagnostics: &mut ParseDiagnostics) -> Option<FilterRule> {
    use self::Token::*;

    let tk = pop_token(tokens, diagnostics, "condition")?;
    match tk {
        StrLit(s) => {
            if s == "clients" {
                return Some(FilterRule::ClientsOnly);
            }

            let pred = parse_predicate(s, tokens, diagnostics)?;
            let op = match pop_token(tokens, diagnostics, "operator")? {
                OP(op) => op,
                token => {
                    push_diag(
                        diagnostics,
                        format!("wrong rule: expected operator, found {:?}", token),
                    );
                    return None;
                }
            };

            let raw_value = match pop_token(tokens, diagnostics, "value")? {
                StrLit(value) => value,
                token => {
                    push_diag(
                        diagnostics,
                        format!("wrong rule: expected value, found {:?}", token),
                    );
                    return None;
                }
            };

            let matcher = parse_matcher(&pred, &raw_value, diagnostics)?;
            Some(FilterRule::Single { pred, op, matcher })
        }

        ANY | ALL => {
            if !expect_token(tokens, LBRACE, diagnostics) {
                return None;
            }

            let mut rules = Vec::new();
            loop {
                if matches!(peek_token(tokens), Some(RBRACE) | Some(EOT) | None) && rules.is_empty()
                {
                    push_diag(diagnostics, "expected condition inside logical group");
                    return None;
                }

                let Some(cond) = parse_cond(tokens, diagnostics) else {
                    return None;
                };
                rules.push(Box::new(cond));

                match peek_token(tokens) {
                    Some(COMMA) => {
                        tokens.pop_front();
                        if matches!(peek_token(tokens), Some(RBRACE)) {
                            push_diag(diagnostics, "expected condition after ','");
                            return None;
                        }
                    }
                    Some(RBRACE) => {
                        tokens.pop_front();
                        break;
                    }
                    Some(token) => {
                        push_diag(
                            diagnostics,
                            format!("expected ',' or ')', found {:?}", token),
                        );
                        return None;
                    }
                    None => {
                        push_diag(diagnostics, "expected ')' to close logical group");
                        return None;
                    }
                }
            }

            if tk == ANY {
                Some(FilterRule::Any(rules))
            } else {
                Some(FilterRule::All(rules))
            }
        }

        NOT => {
            if !expect_token(tokens, LBRACE, diagnostics) {
                return None;
            }

            let cond = parse_cond(tokens, diagnostics)?;
            match pop_token(tokens, diagnostics, "')'")? {
                RBRACE => Some(FilterRule::Not(Box::new(cond))),
                COMMA => {
                    push_diag(diagnostics, "not rule accepts only one condition");
                    None
                }
                token => {
                    push_diag(
                        diagnostics,
                        format!("expected ')' after not(...) condition, found {:?}", token),
                    );
                    None
                }
            }
        }
        _ => {
            push_diag(diagnostics, format!("wrong match: [{:?}]", tk));
            None
        }
    }
}

pub fn scan_tokens<S: AsRef<str>>(rule: S) -> Tokens {
    use self::Token::*;
    macro_rules! append_tok {
        ($tokens:tt, $tk:expr) => {{
            $tokens.push_back($tk);
            //println!("collect [{:?}]", $tk);
        }};
    }

    let mut tokens = Tokens::new();
    let mut chars = rule.as_ref().chars().peekable();
    let metas: HashSet<_> = ['.', ',', ';', ':', '(', ')', '<', '>', '=']
        .iter()
        .cloned()
        .collect();
    let mut need_act = false;

    while let Some(ch) = chars.next() {
        match ch {
            '=' => {
                append_tok!(tokens, OP(Op::Eq));
            }

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

                if do_consume {
                    chars.next();
                }
            }

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
                if do_consume {
                    chars.next();
                }
            }

            '.' => {
                append_tok!(tokens, DOT);
            }
            ',' => {
                append_tok!(tokens, COMMA);
            }
            ';' => {
                append_tok!(tokens, SEMICOLON);
                need_act = false;
            }
            ':' => {
                append_tok!(tokens, COLON);
                need_act = true;
            }
            '(' => {
                append_tok!(tokens, LBRACE);
            }
            ')' => {
                append_tok!(tokens, RBRACE);
            }

            _ if ch.is_whitespace() => {}

            '#' => {
                // Skip comment until end of line
                loop {
                    match chars.peek() {
                        Some('\n') | None => break,
                        _ => {
                            chars.next();
                        }
                    }
                }
            }

            _ => {
                // scan string literal
                let compound_str = matches!(ch, '\'' | '"');

                let mut s = String::new();
                if !compound_str {
                    s.push(ch);
                }
                loop {
                    if compound_str {
                        match chars.peek() {
                            Some(&val) if val != '\'' && val != '"' => {}
                            _ => break,
                        }
                    } else {
                        match chars.peek() {
                            //skip special char
                            Some(val) if !metas.contains(val) => {}
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
                    lowered => append_tok!(tokens, StrLit(lowered.to_string())),
                }
            }
        }
    }

    append_tok!(tokens, EOT);
    tokens
}

pub fn filter_grammar() -> &'static str {
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
mod more_tests {
    use super::*;
    use pretty_assertions::assert_eq; // For better assertion messages

    #[test]
    fn test_scan_tokens_edge_cases() {
        let tokens = scan_tokens("all(name = 'example', id = 0x123);");
        assert_eq!(tokens.len(), 12);
    }

    #[test]
    fn test_parser_ast_equality() {
        let mut tokens = scan_tokens("name = example;");
        let parsed1 = parse_rule(&mut tokens);
        let serialized = serde_json::to_string(&parsed1).unwrap();
        let deserialized: Vec<FilterItem> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed1, Some(deserialized));
    }

    #[test]
    fn test_wildcard_matching() {
        assert!(wild_match("exa?ple", "example"));
        assert!(!wild_match("exa?ple", "samples"));
    }

    #[test]
    fn test_serde_round_trip() {
        let item = FilterItem {
            action: Action::Pin,
            rule: FilterRule::ClientsOnly,
        };
        let serialized = serde_json::to_string(&item).unwrap();
        let deserialized: FilterItem = serde_json::from_str(&serialized).unwrap();
        assert_eq!(item, deserialized);
    }
}
mod tests {
    #[allow(unused_imports)]
    use super::Token::*;
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_macros)]
    macro_rules! append_tok {
        ($tokens:expr, $tk:expr) => {
            $tokens.push_back($tk);
        };
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
        let tokens =
            scan_tokens("any(name =dde?osd, all(geom.x > 2, geom.width < 500));".to_string());
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
        let tokens =
            scan_tokens("any(name = 'inside, this', name = \"name ; any\"): pin".to_string());
        println!("{:?}", tokens);
        assert_eq!(tokens.len(), 13);
    }

    #[test]
    fn test_parse_flow() {
        let mut tokens =
            scan_tokens("any(name =dde?osd*, all(geom.x > 2, geom.width < 500));".to_string());
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

        assert_eq!(
            wild_match("*dde*", "ClutterActor: Clutter Reference Manual"),
            false
        );
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
