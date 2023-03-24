//! The serializable types for the Warg API.
#![deny(missing_docs)]

pub mod content;
pub mod fetch;
pub mod package;
pub mod proof;

/// Implemented on API errors to convert from a `std::error::Error`.
pub trait FromError {
    /// Converts from a generic error type to an API error.
    fn from_error<E: std::error::Error>(error: E) -> Self;
}
