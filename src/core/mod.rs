pub mod error;
pub mod macros;
pub mod state;
pub mod tracing;
pub mod types;
pub mod stack_diff;
pub mod wildcard;
pub mod colorized_output;

pub use error::{WmError, WmResult, CoreResult};
pub use tracing::init_tracing;
pub use types::*;
pub use state::{GlobalState, StateRef, WindowsLayout, create_state_ref};
pub use macros::*;
