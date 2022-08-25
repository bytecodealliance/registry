pub mod log;
pub mod map;

use digest::Digest;

pub trait Digestable {
    fn digest(&self, digest: &mut impl Digest);
}
