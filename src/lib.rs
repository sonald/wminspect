pub mod core;
pub mod dsl;

#[cfg(any(
    feature = "x11",
    feature = "platform-linux",
    feature = "platform-windows",
    feature = "platform-macos"
))]
pub mod platform;

// Re-export important types for external use
pub use crate::core::*;
pub use crate::dsl::*;

#[cfg(feature = "x11")]
pub use crate::platform::*;
