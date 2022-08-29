//! This crate contains forrest data structures.

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rust_2018_idioms, unused_lifetimes)]
#![warn(unused_qualifications, missing_docs)]
#![warn(clippy::all, clippy::panic)]
#![forbid(unsafe_code, clippy::expect_used)]

extern crate alloc;

pub mod log;
pub mod map;

use digest::{Digest, Output};

/// A trait for types which may be cryptographically hashed
pub trait Digestable<D: Digest> {
    /// Update the existing digest
    fn update(&self, digest: &mut D);

    /// Calculate a digest
    fn digest(&self) -> Output<D> {
        let mut digest = D::new();
        self.update(&mut digest);
        digest.finalize()
    }
}

impl<D: Digest, T: Digestable<D>> Digestable<D> for &T {
    fn update(&self, digest: &mut D) {
        T::update(self, digest)
    }
}

impl<D: Digest, T: Digestable<D>> Digestable<D> for &mut T {
    fn update(&self, digest: &mut D) {
        T::update(self, digest)
    }
}

impl<D: Digest> Digestable<D> for alloc::vec::Vec<u8> {
    fn update(&self, digest: &mut D) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl<D: Digest> Digestable<D> for [u8] {
    fn update(&self, digest: &mut D) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl<D: Digest> Digestable<D> for alloc::string::String {
    fn update(&self, digest: &mut D) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl<D: Digest> Digestable<D> for str {
    fn update(&self, digest: &mut D) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl<D: Digest> Digestable<D> for u8 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for u16 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for u32 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for u64 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for u128 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for i8 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for i16 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for i32 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for i64 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for i128 {
    fn update(&self, digest: &mut D) {
        digest.update(self.to_le_bytes());
    }
}

impl<D: Digest> Digestable<D> for () {
    fn update(&self, _: &mut D) {}
}

impl<D: Digest, T0> Digestable<D> for (T0,)
where
    T0: Digestable<D>,
{
    fn update(&self, digest: &mut D) {
        self.0.update(digest);
    }
}

impl<D: Digest, T0, T1> Digestable<D> for (T0, T1)
where
    T0: Digestable<D>,
    T1: Digestable<D>,
{
    fn update(&self, digest: &mut D) {
        self.0.update(digest);
        self.1.update(digest);
    }
}

impl<D: Digest, T0, T1, T2> Digestable<D> for (T0, T1, T2)
where
    T0: Digestable<D>,
    T1: Digestable<D>,
    T2: Digestable<D>,
{
    fn update(&self, digest: &mut D) {
        self.0.update(digest);
        self.1.update(digest);
        self.2.update(digest);
    }
}

impl<D: Digest, T0, T1, T2, T3> Digestable<D> for (T0, T1, T2, T3)
where
    T0: Digestable<D>,
    T1: Digestable<D>,
    T2: Digestable<D>,
    T3: Digestable<D>,
{
    fn update(&self, digest: &mut D) {
        self.0.update(digest);
        self.1.update(digest);
        self.2.update(digest);
        self.3.update(digest);
    }
}
