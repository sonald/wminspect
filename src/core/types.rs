use std::fmt::{self, Debug, Display, Formatter, LowerHex, Write};
use std::cmp::Ordering;
use serde::{Serialize, Deserialize};

/// Helper type to format vec of window IDs in hex
pub struct HexedVec<'a, T: 'a>(&'a Vec<T>);

impl<'a, T: Debug + LowerHex> Debug for HexedVec<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut has_next = false;
        let mut s = String::new();
        write!(&mut s, "[")?;
        for t in self.0 {
            let prefix = if has_next { ", " } else { "" };
            write!(&mut s, "{}{:#x}", prefix, t)?;
            has_next = true;
        }
        write!(&mut s, "]")?;
        
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Display for Geometry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}x{}+{}+{}", self.width, self.height, self.x, self.y)
    }
}

impl Geometry {
    // Commented out due to XCB API changes
    // #[cfg(feature = "x11")]
    // pub fn update_with_configure(&mut self, cne: &xcb::x::ConfigureNotifyEvent) {
    //     self.x = cne.x();
    //     self.y = cne.y();
    //     self.width = cne.width();
    //     self.height = cne.height();
    // }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq)]
pub enum MapState {
    Unmapped,
    Viewable,
    Unviewable,
}

impl PartialEq for MapState {
    fn eq(&self, other: &Self) -> bool {
        (*self as i32) == (*other as i32)
    }
}

impl Display for MapState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            MapState::Unmapped => "Unmapped",
            MapState::Unviewable => "Unviewable",
            MapState::Viewable => "Viewable"
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Attributes {
    pub override_redirect: bool,
    pub map_state: MapState,
}

impl Display for Attributes {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", 
               if self.override_redirect { "OR " } else { "" }, 
               self.map_state)
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub id: u32,
    pub name: String,
    pub attrs: Attributes,
    pub geom: Geometry,
    pub valid: bool,
}

impl Eq for Window {}

impl Ord for Window {
    fn cmp(&self, other: &Window) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Window {
    fn partial_cmp(&self, other: &Window) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.id == other.id
    }
}

impl Display for Window {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let id = format!("0x{:x}", self.id);
        write!(f, "{}({}) {} {}", id, self.name, self.geom, self.attrs)
    }
}

#[derive(Debug, Clone)]
pub enum Condition {
    Colorful,
    MappedOnly,
    OmitHidden,
    NoSpecial,
    ShowDiff,
    ClientsOnly,
    NoOverrideRedirect,
}

pub type WindowId = u32;
pub type WindowStackView = Vec<WindowId>;
pub type WindowListView = std::collections::HashSet<WindowId>;

#[derive(Clone)]
pub enum Message {
    #[cfg(feature = "x11")]
    LastConfigureEvent(String), // Simplified to avoid clone issues
    Reset,
    Quit,
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        use self::Message::*;
        match self {
            #[cfg(feature = "x11")]
            LastConfigureEvent(raw) => {
                write!(f, "Message::LastConfigureEvent({})", raw)
            },
            Reset => write!(f, "Message::Reset"),
            Quit => write!(f, "Message::Quit"),
        }
    }
}

// Commented out due to XCB API changes
// #[cfg(feature = "x11")]
// pub enum XcbRequest<'a> {
//     GWA(xcb::x::GetWindowAttributesCookie),
//     GE(xcb::x::GetGeometryCookie),
//     GP(xcb::x::GetPropertyCookie),
//     GWN(xcb_util::ewmh::GetWmNameCookie<'a>),
// }
