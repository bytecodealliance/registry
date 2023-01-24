use alloc::sync::Arc;
use alloc::vec::Vec;

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::fork::Fork;
use super::link::Link;
use super::path::Path;
use super::proof::Proof;

pub enum Node<D: SupportedDigest> {
    Leaf(Hash<D>),
    Fork(Fork<D>),
}

impl<D: SupportedDigest> Clone for Node<D> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.clone()),
            Self::Fork(node) => Self::Fork(node.clone()),
        }
    }
}

impl<D: SupportedDigest> Default for Node<D> {
    fn default() -> Self {
        Self::Fork(Fork::default())
    }
}

impl<D: SupportedDigest> Node<D> {
    pub fn hash(&self) -> Hash<D> {
        match self {
            Node::Leaf(hash) => hash.clone(),
            Node::Fork(fork) => fork.hash(),
        }
    }

    pub fn prove<V: VisitBytes>(&self, mut path: Path<D>) -> Option<Proof<D, V>> {
        match (path.next(), self) {
            (Some(idx), Self::Fork(fork)) => {
                let mut proof = fork[idx].as_ref()?.node().prove(path)?;
                let peer = fork[idx.opposite()].as_ref().map(|link| link.hash());

                proof.push(peer.cloned());

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
    pub fn insert(&self, path: &mut Path<D>, leaf: Hash<D>) -> (Self, bool) {
        match path.next() {
            // We are at the end of the path. Save the leaf.
            None => (Node::Leaf(leaf), matches!(self, Node::Fork(..))),

            // We are not at the end of the path. Recurse...
            Some(index) => match self.clone() {
                Node::Fork(mut fork) => {
                    // Choose the branch on the specified side.
                    let node = fork[index]
                        .as_ref()
                        .map(|link| link.node().clone())
                        .unwrap_or_default();

                    // Replace its value recursively.
                    let (node, new) = node.insert(path, leaf);
                    fork[index] = Some(Arc::new(Link::new(node)));
                    (Node::Fork(fork), new)
                }

                _ => unreachable!(),
            },
        }
    }
}
