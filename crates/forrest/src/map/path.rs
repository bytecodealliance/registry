// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use core::iter::FusedIterator;

use digest::{Digest, Output};

pub struct Path<D: Digest> {
    all: Output<D>,
    lhs: usize,
    rhs: usize,
}

impl<D: Digest> Path<D> {
    fn get(&self, at: usize) -> usize {
        let shift = 7 - at % 8;
        let byte = at / 8;

        (usize::from(self.all[byte]) >> shift) & 1
    }
}

impl<D: Digest, K: AsRef<[u8]>> From<K> for Path<D> {
    fn from(key: K) -> Self {
        let all = D::digest(key);

        Self {
            lhs: 0,
            rhs: all.as_ref().len() * 8,
            all,
        }
    }
}

impl<D: Digest> FusedIterator for Path<D> {}
impl<D: Digest> ExactSizeIterator for Path<D> {}

impl<D: Digest> DoubleEndedIterator for Path<D> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.lhs == self.rhs {
            return None;
        }

        self.rhs -= 1;
        Some(self.get(self.rhs))
    }
}

impl<D: Digest> Iterator for Path<D> {
    type Item = usize;

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

#[test]
#[allow(clippy::identity_op)]
fn test() {
    let mut path = Path::<sha2::Sha256>::from("foo");
    let mut hash = sha2::Sha256::digest("foo").into_iter().map(usize::from);

    for _ in 0..hash.len() / 2 {
        let lhs = hash.next().unwrap();
        assert_eq!((lhs >> 7) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 6) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 5) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 4) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 3) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 2) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 1) & 0b1, path.next().unwrap());
        assert_eq!((lhs >> 0) & 0b1, path.next().unwrap());

        let rhs = hash.next_back().unwrap();
        assert_eq!((rhs >> 0) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 1) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 2) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 3) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 4) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 5) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 6) & 0b1, path.next_back().unwrap());
        assert_eq!((rhs >> 7) & 0b1, path.next_back().unwrap());
    }

    assert!(hash.next().is_none());
    assert!(path.next().is_none());
}
