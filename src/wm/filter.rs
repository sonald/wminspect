use super::wm::*;

#[derive(Debug, Clone)]
enum Condition {
    Colorful,
    MappedOnly,
    OmitHidden,
}

type FilterFunction = Box<Fn(&Window) -> bool + Send>;

pub struct Filter{
    options: Vec<Condition>,
    applys: Vec<FilterFunction>
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
}

/// rule: and(attr(map_state=Viewable), geom(x>2))
pub fn parse_filter(rule: String) -> Filter {
    let mut filter = Filter {options: Vec::new(), applys: Vec::new()};

    //filter.applys.push(Box::new(move |w: &Window| w.attrs.map_state == MapState::Viewable));

    filter
}

