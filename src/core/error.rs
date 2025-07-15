use thiserror::Error;

#[derive(Error, Debug)]
pub enum WmError {
    #[error("XCB connection error: {0}")]
    XcbConnection(String),
    
    #[error("XCB query error: {0}")]
    XcbQuery(String),
    
    #[error("XCB error: {0}")]
    XcbError(String),
    
    #[error("Window not found: {0:#x}")]
    WindowNotFound(u32),
    
    #[error("Invalid window attribute: {0}")]
    InvalidAttribute(String),
    
    #[error("Filter parsing error: {0}")]
    FilterParsing(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Runtime error: {0}")]
    Runtime(String),
}

impl From<serde_json::Error> for WmError {
    fn from(err: serde_json::Error) -> Self {
        WmError::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for WmError {
    fn from(err: bincode::Error) -> Self {
        WmError::Serialization(err.to_string())
    }
}

#[cfg(feature = "x11")]
// Commented out due to XCB API changes
// impl From<xcb::Error> for WmError {
//     fn from(err: xcb::Error) -> Self {
//         WmError::XcbError(err.to_string())
//     }
// }

/// Result type alias for window manager operations
pub type WmResult<T> = Result<T, WmError>;

/// Result type alias for core operations
pub type CoreResult<T> = Result<T, WmError>;

/// Result type alias for DSL operations
pub type DslResult<T> = Result<T, WmError>;

/// Result type alias for UI operations
pub type UiResult<T> = Result<T, WmError>;

/// Result type alias for platform operations
pub type PlatformResult<T> = Result<T, WmError>;
