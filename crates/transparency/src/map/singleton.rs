use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::{
    map::hash_branch,
    path::{ReversePath, Side},
};

#[derive(Debug)]
pub struct Singleton<D: SupportedDigest, K: VisitBytes + Clone> {
    pub key: K,
    pub value: Hash<D>,
    pub height: usize,
    pub side: Side,
}

impl<D: SupportedDigest, K: VisitBytes + Clone> Singleton<D, K> {
    pub fn new(key: K, value: Hash<D>, height: usize, side: Side) -> Self {
        Self {
            key,
            value,
            height,
            side,
        }
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn hash(&self) -> Hash<D> {
        let mut hash = self.value.clone();
        let mut reversed: ReversePath<D> = ReversePath::new(&self.key);
        for n in 0..self.height {
            hash = match reversed.next() {
                Some(side) => match side {
                    Side::Left => hash_branch(&hash, &D::empty_tree_hash(n)),
                    Side::Right => hash_branch(&D::empty_tree_hash(n), &hash),
                },
                None => hash,
            };
        }
        hash
    }
}

impl<D: SupportedDigest, K: VisitBytes + Clone> Clone for Singleton<D, K> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            height: self.height,
            side: self.side,
        }
    }
}
