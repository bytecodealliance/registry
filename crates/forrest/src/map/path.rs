use core::iter::FusedIterator;

use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

pub struct Path<D: SupportedDigest> {
    all: Hash<D>,
    lhs: usize,
    rhs: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Side corresponding to the bit value 0
    Left,
    /// Side corresponding to the bit value 1
    Right,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

impl<D: SupportedDigest> Path<D> {
    pub(crate) fn new<K: VisitBytes>(key: K) -> Self {
        let all = Hash::of(&key);

        Self {
            lhs: 0,
            rhs: all.len() * 8,
            all: all.into(),
        }
    }

    fn get(&self, at: usize) -> Side {
        let shift = 7 - at % 8;
        let byte = at / 8;

        let bit_value = (self.all.bytes()[byte] >> shift) & 1;
        let is_right = bit_value == 1;
        if is_right {
            Side::Right
        } else {
            Side::Left
        }
    }
}

impl<D: SupportedDigest> FusedIterator for Path<D> {}
impl<D: SupportedDigest> ExactSizeIterator for Path<D> {}

impl<D: SupportedDigest> DoubleEndedIterator for Path<D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.lhs == self.rhs {
            return None;
        }

        self.rhs -= 1;
        Some(self.get(self.rhs))
    }
}

impl<D: SupportedDigest> Iterator for Path<D> {
    type Item = Side;

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.rhs - self.lhs;
        (size, Some(size))
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.lhs == self.rhs {
            return None;
        }

        self.lhs += 1;
        Some(self.get(self.lhs - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warg_crypto::hash::Sha256;

    fn side(num: u8) -> Side {
        match num {
            0 => Side::Left,
            1 => Side::Right,
            _ => panic!(),
        }
    }

    #[test]
    #[allow(clippy::identity_op)]
    fn test_forwards() {
        let mut path = Path::<Sha256>::new("foo");
        let hash: Hash<Sha256> = Hash::of(&"foo");
        let mut bytes = hash.bytes().iter();

        for _ in 0..hash.len() {
            let lhs = *bytes.next().unwrap();
            assert_eq!(side((lhs >> 7) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 6) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 5) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 4) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 3) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 2) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 1) & 1), path.next().unwrap());
            assert_eq!(side((lhs >> 0) & 1), path.next().unwrap());
        }

        assert!(bytes.next().is_none());
        assert!(path.next().is_none());
    }

    #[test]
    #[allow(clippy::identity_op)]
    fn test_backwards() {
        let mut path = Path::<Sha256>::new("foo");
        let hash: Hash<Sha256> = Hash::of(&"foo");
        let mut bytes = hash.bytes().iter();

        for _ in 0..hash.len() {
            let rhs = *bytes.next_back().unwrap();
            assert_eq!(side((rhs >> 0) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 1) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 2) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 3) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 4) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 5) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 6) & 1), path.next_back().unwrap());
            assert_eq!(side((rhs >> 7) & 1), path.next_back().unwrap());
        }

        assert!(bytes.next().is_none());
        assert!(path.next().is_none());
    }
}
