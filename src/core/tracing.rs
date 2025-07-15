pub use tracing::{debug, error, info, trace, warn, event, Level, span, instrument};
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing for the application
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("wminspect=debug".parse().unwrap()))
        .init();
}

/// Convenience macro for debug tracing that includes file and line info
#[macro_export]
macro_rules! wm_trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            tracing::debug!(target: "wminspect", file = file!(), line = line!(), $($arg)*);
        }
    };
}

/// Macro for error tracing
#[macro_export]
macro_rules! wm_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "wminspect", $($arg)*);
    };
}

/// Macro for info tracing
#[macro_export]
macro_rules! wm_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "wminspect", $($arg)*);
    };
}

/// Macro for warning tracing
#[macro_export]
macro_rules! wm_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "wminspect", $($arg)*);
    };
}

/// Create a span for structured tracing
#[macro_export]
macro_rules! wm_span {
    ($level:expr, $name:expr) => {
        use crate::core::tracing::{span, Level};
        span!($level, $name)
    };
    ($level:expr, $name:expr, $($field:tt)*) => {
        use crate::core::tracing::{span, Level};
        span!($level, $name, $($field)*)
    };
}

/// Enter a span for the current scope
#[macro_export]
macro_rules! wm_span_enter {
    ($span:expr) => {
        let _enter = $span.enter();
    };
}
