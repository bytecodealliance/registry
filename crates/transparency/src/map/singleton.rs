use warg_crypto::hash::{Hash, SupportedDigest};

use super::{
    map::hash_branch,
    path::{Path, ReversePath, Side},
};

#[derive(Debug)]
pub struct Singleton<D: SupportedDigest> {
    key: Hash<D>,
    value: Hash<D>,
}

impl<D: SupportedDigest> Singleton<D> {
    pub fn new(
        key: Hash<D>,
        value: Hash<D>,
        path: &Path<D>,
        reversed: &mut ReversePath<D>,
    ) -> Self {
        let cur = 256 - path.index();
        let mut hash = value;
        for n in 0..cur {
            hash = match reversed.next() {
                Some(side) => match side {
                    Side::Left => hash_branch(Some(hash.clone()), Some(D::empty_tree_hash(n))),
                    Side::Right => hash_branch(Some(D::empty_tree_hash(n)), Some(hash.clone())),
                },
                None => hash.clone(),
            };
        }
        Self { key, value: hash }
    }

    pub fn key(&self) -> Hash<D> {
        self.key.clone()
    }

    pub fn hash(&self) -> Hash<D> {
        self.value.clone()
    }
}
impl<D: SupportedDigest> Clone for Singleton<D> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
        }
    }
}
