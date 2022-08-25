pub mod log;
pub mod map;

use digest::{Digest, Output};

pub trait Digestable {
    fn update(&self, digest: &mut impl Digest);

    fn digest<D: Digest>(&self) -> Output<D> {
        let mut digest = D::new();
        self.update(&mut digest);
        digest.finalize()
    }
}

impl Digestable for Vec<u8> {
    fn update(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl Digestable for [u8] {
    fn update(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl Digestable for String {
    fn update(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl Digestable for &str {
    fn update(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl Digestable for u8 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u16 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u32 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u64 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u128 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i8 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i16 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i32 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i64 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i128 {
    fn update(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for () {
    fn update(&self, _: &mut impl Digest) {}
}

impl<T0> Digestable for (T0,)
where
    T0: Digestable,
{
    fn update(&self, digest: &mut impl Digest) {
        self.0.update(digest);
    }
}

impl<T0, T1> Digestable for (T0, T1)
where
    T0: Digestable,
    T1: Digestable,
{
    fn update(&self, digest: &mut impl Digest) {
        self.0.update(digest);
        self.1.update(digest);
    }
}

impl<T0, T1, T2> Digestable for (T0, T1, T2)
where
    T0: Digestable,
    T1: Digestable,
    T2: Digestable,
{
    fn update(&self, digest: &mut impl Digest) {
        self.0.update(digest);
        self.1.update(digest);
        self.2.update(digest);
    }
}

impl<T0, T1, T2, T3> Digestable for (T0, T1, T2, T3)
where
    T0: Digestable,
    T1: Digestable,
    T2: Digestable,
    T3: Digestable,
{
    fn update(&self, digest: &mut impl Digest) {
        self.0.update(digest);
        self.1.update(digest);
        self.2.update(digest);
        self.3.update(digest);
    }
}
