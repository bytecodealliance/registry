//! This crate contains forrest data structures.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms, unused_lifetimes)]
#![warn(unused_qualifications, missing_docs)]
#![warn(clippy::all, clippy::panic)]
#![forbid(unsafe_code, clippy::expect_used)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod log;
pub mod map;
