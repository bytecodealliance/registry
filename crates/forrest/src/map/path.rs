use core::iter::FusedIterator;

use warg_crypto::hash::{Hash, SupportedDigest};

use super::{link::Link, node::Node};

pub struct Path<D: SupportedDigest> {
    all: Hash<D>,
    lhs: usize,
    rhs: usize,
}

impl<D: SupportedDigest> Path<D> {
    fn get(&self, at: usize) -> bool {
        let shift = 7 - at % 8;
        let byte = at / 8;

        ((self.all[byte] >> shift) & 1) == 1
    }

    pub fn hash<V: AsRef<[u8]>>(&self, value: &V) -> Hash<D> {
        D::new_with_prefix(&[0xff])
            .chain_update(&*self.all)
            .chain_update(value)
            .finalize()
            .into()
    }

    pub fn link<K, V: AsRef<[u8]>>(&self, node: Node<D, K, V>) -> Link<D, K, V> {
        let hash = match &node {
            Node::Leaf(leaf) => self.hash(&leaf.1),
            Node::Fork(fork) => fork.hash(),
        };

        Link { hash, node }
    }
}

impl<D: SupportedDigest, K: AsRef<[u8]>> From<K> for Path<D> {
    fn from(key: K) -> Self {
        let all = D::digest(key);

        Self {
            lhs: 0,
            rhs: all.as_ref().len() * 8,
            all: all.into(),
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
    type Item = bool;

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
    use warg_crypto::hash::Digest;
    use warg_crypto::hash::Sha256;

    #[test]
    #[allow(clippy::identity_op)]
    fn test() {
        let mut path = Path::<Sha256>::from("foo");
        let mut hash = Sha256::digest("foo").into_iter().map(usize::from);

        for _ in 0..hash.len() / 2 {
            let lhs = hash.next().unwrap();
            assert_eq!((lhs >> 7) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 6) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 5) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 4) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 3) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 2) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 1) & 1 == 1, path.next().unwrap());
            assert_eq!((lhs >> 0) & 1 == 1, path.next().unwrap());

            let rhs = hash.next_back().unwrap();
            assert_eq!((rhs >> 0) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 1) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 2) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 3) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 4) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 5) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 6) & 1 == 1, path.next_back().unwrap());
            assert_eq!((rhs >> 7) & 1 == 1, path.next_back().unwrap());
        }

        assert!(hash.next().is_none());
        assert!(path.next().is_none());
    }
}
