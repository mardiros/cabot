pub use super::errors::CabotError;

/// Result used by method that can failed.
pub type CabotResult<T> = Result<T, CabotError>;
