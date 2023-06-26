use alloc::sync::Arc;
use alloc::vec::Vec;
use std::fmt::Debug;

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::fork::Fork;
use super::link::Link;
use super::path::{Path, Side};
use super::proof::Proof;
use super::singleton::Singleton;

#[derive(Debug)]
pub enum Node<D: SupportedDigest> {
    Leaf(Hash<D>),
    Fork(Fork<D>),
    Singleton(Singleton<D>),
    Empty(usize),
}

impl<D: SupportedDigest> Clone for Node<D> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.clone()),
            Self::Fork(node) => Self::Fork(node.clone()),
            Self::Singleton(s) => Self::Singleton(s.clone()),
            Self::Empty(height) => Self::Empty(*height),
        }
    }
}

impl<D: SupportedDigest> Node<D> {
    pub fn hash(&self) -> Hash<D> {
        match self {
            Node::Leaf(hash) => hash.clone(),
            Node::Fork(fork) => fork.hash(),
            Node::Singleton(singleton) => singleton.hash(),
            Node::Empty(height) => D::empty_tree_hash(*height).to_owned(),
        }
    }

    pub fn prove<K: VisitBytes, V: VisitBytes + Clone>(
        &self,
        mut path: Path<'_, D>,
    ) -> Option<Proof<D, K, V>> {
        match (path.next(), self) {
            (Some(_), Self::Singleton(singleton)) => {
                if singleton.key() == path.hash() {
                    Some(Proof::new(Vec::new()))
                } else {
                    None
                }
            }
            (Some(_), Self::Empty(_)) => None,
            (Some(idx), Self::Fork(fork)) => {
                let mut proof = fork[idx].as_ref().node().prove(path)?;
                let peer = fork[idx.opposite()].as_ref().hash();
                proof.push(Some(peer.clone()));
                Some(proof)
            }

            (None, Self::Leaf(_)) => Some(Proof::new(Vec::new())),

            _ => None,
        }
    }

    /// A recursive function for setting the value in the tree.
    ///
    /// Returns:
    ///   * the new node that must replace the current node.
    ///   * whether or not this is a new entry in the map.
    pub fn insert(&self, path: &mut Path<'_, D>, value: Hash<D>) -> (Self, bool) {
        match path.next() {
            // We are at the end of the path. Save the leaf.
            None => (
                Node::Leaf(value),
                matches!(self, Node::Empty(_)) || matches!(self, Node::Singleton(_)),
            ),

            // We are not at the end of the path. Recurse...
            Some(index) => match self.clone() {
                Node::Empty(_) => {
                    if path.index() == 1 {
                        let singleton = Node::Singleton(Singleton::new(
                            path.hash().clone(),
                            value,
                            path.height(),
                        ));
                        match index {
                            Side::Left => {
                                let fork = Fork::new(
                                    Arc::new(Link::new(singleton)),
                                    Arc::new(Link::new(Node::Empty(path.height()))),
                                );
                                return (Node::Fork(fork), true);
                            }
                            Side::Right => {
                                let fork = Fork::new(
                                    Arc::new(Link::new(Node::Empty(path.height()))),
                                    Arc::new(Link::new(singleton)),
                                );
                                return (Node::Fork(fork), true);
                            }
                        }
                    }
                    let singleton = Node::Singleton(Singleton::new(
                        path.hash().clone(),
                        value,
                        path.height() + 1,
                    ));
                    (singleton, true)
                }
                Node::Fork(mut fork) => {
                    // Choose the branch on the specified side.
                    let (node, new) = fork[index].as_ref().node().insert(path, value);
                    fork[index] = Arc::new(Link::new(node));
                    (Node::Fork(fork), new)
                }
                Node::Singleton(singleton) => {
                    singleton.insert(path, path.hash().clone(), value, index)
                }
                Node::Leaf(_) => (Node::Leaf(value), false),
            },
        }
    }
}
