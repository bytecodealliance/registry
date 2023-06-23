use warg_crypto::hash::{Hash, SupportedDigest};

use super::{
    fork::Fork,
    link::Link,
    map::hash_branch,
    node::Node,
    path::{Path, ReversePath, Side},
};
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct Singleton<D: SupportedDigest> {
    pub key: Hash<D>,
    pub value: Hash<D>,
    pub height: usize,
    pub side: Side,
}

impl<D: SupportedDigest> Singleton<D> {
    pub fn new(key: Hash<D>, value: Hash<D>, height: usize, side: Side) -> Self {
        Self {
            key,
            value,
            height,
            side,
        }
    }

    pub fn key(&self) -> &Hash<D> {
        &self.key
    }

    pub fn hash(&self) -> Hash<D> {
        let mut hash = self.value.clone();
        let mut reversed: ReversePath<D> = ReversePath::new(self.key.clone());
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

    pub fn insert(
        &self,
        path: &mut Path<D>,
        key: Hash<D>,
        value: Hash<D>,
        cur_side: Side,
    ) -> (Node<D>, bool) {
        if self.key() == &key {
            let new_singleton = Singleton::new(key, value, path.height() + 1, cur_side);
            (Node::Singleton(new_singleton), false)
        } else if self.side != cur_side {
            let node = Node::Singleton(Singleton::new(key, value, path.height(), cur_side));
            let original = Node::Singleton(Singleton::new(
                self.key.clone(),
                self.value.clone(),
                path.height(),
                cur_side.opposite(),
            ));
            let fork = match cur_side {
                Side::Left => Fork::new(Arc::new(Link::new(node)), Arc::new(Link::new(original))),
                Side::Right => Fork::new(Arc::new(Link::new(original)), Arc::new(Link::new(node))),
            };
            (Node::Fork(fork), false)
        } else {
            let cur_path = Path::new(self.key.clone());
            let cur_index = path.index();
            let fork = match cur_side {
                Side::Left => {
                    let pre_insert = Node::Singleton(Singleton::new(
                        self.key.clone(),
                        self.value.clone(),
                        self.height - 1,
                        cur_path.get(cur_index),
                    ));
                    let (down_one, _) = pre_insert.insert(path, value);

                    Fork::new(
                        Arc::new(Link::new(down_one)),
                        Arc::new(Link::new(Node::Empty(256 - cur_index))),
                    )
                }
                Side::Right => {
                    let pre_insert = Node::Singleton(Singleton::new(
                        self.key.clone(),
                        self.value.clone(),
                        self.height - 1,
                        cur_path.get(cur_index),
                    ));
                    let (down_one, _) = pre_insert.insert(path, value);
                    Fork::new(
                        Arc::new(Link::new(Node::Empty(256 - cur_index))),
                        Arc::new(Link::new(down_one)),
                    )
                }
            };
            (Node::Fork(fork), true)
        }
    }
}

impl<D: SupportedDigest> Clone for Singleton<D> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            value: self.value.clone(),
            height: self.height,
            side: self.side,
        }
    }
}
