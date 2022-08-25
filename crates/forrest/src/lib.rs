pub mod log;
pub mod map;

use digest::Digest;

pub trait Digestable {
    fn digest(&self, digest: &mut impl Digest);
}

impl Digestable for Vec<u8> {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl Digestable for [u8] {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self);
    }
}

impl Digestable for String {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl Digestable for &str {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update((self.len() as u64).to_le_bytes());
        digest.update(self.as_bytes());
    }
}

impl Digestable for u8 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u16 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u32 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u64 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for u128 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i8 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i16 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i32 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i64 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for i128 {
    fn digest(&self, digest: &mut impl Digest) {
        digest.update(self.to_le_bytes());
    }
}

impl Digestable for () {
    fn digest(&self, _: &mut impl Digest) {}
}

impl<T0> Digestable for (T0,)
where
    T0: Digestable,
{
    fn digest(&self, digest: &mut impl Digest) {
        self.0.digest(digest);
    }
}

impl<T0, T1> Digestable for (T0, T1)
where
    T0: Digestable,
    T1: Digestable,
{
    fn digest(&self, digest: &mut impl Digest) {
        self.0.digest(digest);
        self.1.digest(digest);
    }
}

impl<T0, T1, T2> Digestable for (T0, T1, T2)
where
    T0: Digestable,
    T1: Digestable,
    T2: Digestable,
{
    fn digest(&self, digest: &mut impl Digest) {
        self.0.digest(digest);
        self.1.digest(digest);
        self.2.digest(digest);
    }
}

impl<T0, T1, T2, T3> Digestable for (T0, T1, T2, T3)
where
    T0: Digestable,
    T1: Digestable,
    T2: Digestable,
    T3: Digestable,
{
    fn digest(&self, digest: &mut impl Digest) {
        self.0.digest(digest);
        self.1.digest(digest);
        self.2.digest(digest);
        self.3.digest(digest);
    }
}
