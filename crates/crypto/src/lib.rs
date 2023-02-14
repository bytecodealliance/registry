mod encoding;
pub mod hash;
pub mod signing;

/// Module for prefix encoding
pub mod prefix;
mod visit_bytes;

use anyhow::Error;

pub use encoding::{Encode, Signable};
pub use visit_bytes::{ByteVisitor, VisitBytes};

pub trait Decode: Sized {
    fn decode(bytes: &[u8]) -> Result<Self, Error>;
}
