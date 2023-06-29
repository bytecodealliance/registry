use warg_crypto::hash::{Hash, SupportedDigest};

pub struct Path<'a, D: SupportedDigest> {
    hash: &'a Hash<D>,
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

impl<'a, D: SupportedDigest> Path<'a, D> {
    pub(crate) fn new(hash: &'a Hash<D>) -> Self {
        Self { hash, index: 0 }
    }

    pub fn get(&self, at: usize) -> Side {
        let shift = 7 - at % 8;
        let byte = at / 8;

        let bit_value = (self.hash.bytes()[byte] >> shift) & 1;
        let is_right = bit_value == 1;
        if is_right {
            Side::Right
        } else {
            Side::Left
        }
    }

    pub fn hash(&self) -> &Hash<D> {
        self.hash
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn height(&self) -> usize {
        256 - self.index
    }
}

impl<D: SupportedDigest> Iterator for Path<'_, D> {
    type Item = Side;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.hash.bit_len() {
            return None;
        }

        self.index += 1;
        Some(self.get(self.index - 1))
    }
}

pub struct ReversePath<D: SupportedDigest> {
    hash: Hash<D>,
    index: usize,
}

impl<D: SupportedDigest> Clone for ReversePath<D> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            index: self.index,
        }
    }
}

impl<D: SupportedDigest> ReversePath<D> {
    pub(crate) fn new(hash: Hash<D>) -> Self {
        let start = hash.bytes().len() * 8;

        Self { index: start, hash }
    }

    fn get(&self, at: usize) -> Side {
        let shift = 7 - at % 8;
        let byte = at / 8;
        let bit_value = (self.hash.bytes()[byte] >> shift) & 1;
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
        let hash: Hash<Sha256> = Hash::of("foo");
        let mut path = Path::<Sha256>::new(&hash);
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
        let mut path = ReversePath::<Sha256>::new(Hash::of("foo"));
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
