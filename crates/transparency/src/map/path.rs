use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

pub struct Path<D: SupportedDigest, K: VisitBytes + Clone> {
    key: K,
    all: Hash<D>,
    index: usize,
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

impl<D: SupportedDigest, K: VisitBytes + Clone> Path<D, K> {
    pub(crate) fn new(key: K) -> Self {
        let all = Hash::of(&key);

        Self { key, index: 0, all }
    }

    pub fn key(&self) -> &K {
        &self.key
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

    pub fn index(&self) -> usize {
        self.index
    }
}

impl<D: SupportedDigest, K: VisitBytes + Clone> Iterator for Path<D, K> {
    type Item = Side;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.all.bit_len() {
            return None;
        }

        self.index += 1;
        Some(self.get(self.index - 1))
    }
}

pub struct ReversePath<D: SupportedDigest> {
    all: Hash<D>,
    index: usize,
}

impl<D: SupportedDigest> Clone for ReversePath<D> {
    fn clone(&self) -> Self {
        Self {
            all: self.all.clone(),
            index: self.index,
        }
    }
}

impl<D: SupportedDigest> ReversePath<D> {
    pub(crate) fn new<K: VisitBytes>(key: &K) -> Self {
        let all = Hash::of(key);
        let start = all.len() * 8;

        Self { index: start, all }
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
impl<D: SupportedDigest> Iterator for ReversePath<D> {
    type Item = Side;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            return None;
        }

        self.index -= 1;
        Some(self.get(self.index))
    }
}
#[cfg(test)]
#[allow(clippy::panic)]
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
        let mut path = Path::<Sha256, &str>::new("foo");
        let hash: Hash<Sha256> = Hash::of("foo");
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
        let mut path = ReversePath::<Sha256>::new(&"foo");
        let hash: Hash<Sha256> = Hash::of("foo");
        let mut bytes = hash.bytes().iter();

        for _ in 0..hash.len() {
            let rhs = *bytes.next_back().unwrap();

            assert_eq!(side((rhs >> 0) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 1) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 2) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 3) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 4) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 5) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 6) & 1), path.next().unwrap());
            assert_eq!(side((rhs >> 7) & 1), path.next().unwrap());
        }

        assert!(bytes.next().is_none());
        assert!(path.next().is_none());
    }
}
