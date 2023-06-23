use warg_crypto::{
    hash::{Hash, SupportedDigest},
};

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
    ) -> (Node<D>, bool) {
        if self.key() == &key {
            let new_singleton = Singleton::new(key, value, path.height() + 1, path.get(path.index()));
            (Node::Singleton(new_singleton), false)
        } else if self.side != path.get(path.index()) {
            let node = Node::Singleton(Singleton::new(key, value, path.height(), path.get(path.index())));
            let original = Node::Singleton(Singleton::new(
                self.key.clone(),
                self.value.clone(),
                path.height(),
                path.get(path.index()).opposite(),
            ));
            let fork = match path.get(path.index()) {
                Side::Left => Fork::new(Arc::new(Link::new(node)), Arc::new(Link::new(original))),
                Side::Right => Fork::new(Arc::new(Link::new(original)), Arc::new(Link::new(node))),
            };
            (Node::Fork(fork), false)
        } else {
            let fork = match path.get(path.index()) {
                Side::Left => {
                    let (down_one, _) = Node::Singleton(Singleton::new(
                        self.key.clone(),
                        self.value.clone(),
                        self.height - 1,
                        path.get(path.index()),
                    ))
                    .insert(path, value);
                    Fork::new(
                        Arc::new(Link::new(down_one)),
                        Arc::new(Link::new(Node::Empty(path.height() - 1))),
                    )
                }
                Side::Right => {
                    let (down_one, _) = Node::Singleton(Singleton::new(
                        self.key.clone(),
                        self.value.clone(),
                        self.height - 1,
                        path.get(path.index()),
                    ))
                    .insert(path, value);
                    Fork::new(
                        Arc::new(Link::new(Node::Empty(path.height() - 1))),
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
