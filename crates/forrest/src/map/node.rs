// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use core::marker::PhantomData;

use alloc::rc::Rc;
use alloc::vec::Vec;

use digest::Digest;

use super::hash::Hash;
use super::link::Link;
use super::path::Path;
use super::proof::Proof;

pub enum Node<D: Digest, K, V> {
    Leaf(Rc<(K, V)>),
    Fork([Option<Rc<Link<D, K, V>>>; 2]),
}

impl<D: Digest, K, V> Clone for Node<D, K, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf(leaf) => Self::Leaf(leaf.clone()),
            Self::Fork(node) => Self::Fork(node.clone()),
        }
    }
}

impl<D: Digest, K, V> Default for Node<D, K, V> {
    fn default() -> Self {
        Self::Fork([None, None])
    }
}

impl<D: Digest, K, V> Node<D, K, V> {
    pub fn get(&self, mut rhs: Path<D>) -> Option<&V> {
        match (rhs.next(), self) {
            (Some(idx), Self::Fork(fork)) => fork[idx].as_ref()?.node.get(rhs),
            (None, Self::Leaf(leaf)) => Some(&leaf.1),
            _ => None,
        }
    }

    pub fn prove(&self, mut rhs: Path<D>) -> Option<Proof<D, &Hash<D>, &V>> {
        match (rhs.next(), self) {
            (Some(idx), Self::Fork(fork)) => {
                let mut proof = fork[idx].as_ref()?.node.prove(rhs)?;
                let peer = fork[(idx + 1) % 2].as_ref().map(|link| &link.hash);

                // This is an optimization. The size of a proof is always
                // known: it is the number of bits in the digest. Therefore,
                // we can skip all leading nodes with no peer. The validator,
                // can reconstruct this.
                if !proof.peers.is_empty() || peer.is_some() {
                    proof.peers.push(peer);
                }

                Some(proof)
            }

            (None, Self::Leaf(leaf)) => Some(Proof {
                digest: PhantomData,
                peers: Vec::new(),
                value: &leaf.1,
            }),

            _ => None,
        }
    }
}

impl<D, K, V> Node<D, K, V>
where
    D: Digest,
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    /// A recursive function for setting the value in the tree.
    ///
    /// Returns:
    ///   * the new node that must replace the current node.
    ///   * whether or not this is a new entry in the map.
    pub fn insert(&self, mut rhs: Path<D>, leaf: (K, V)) -> (Self, bool) {
        match rhs.next() {
            // We are at the end of the path. Save the leaf.
            None => (Node::Leaf(leaf.into()), matches!(self, Node::Fork(..))),

            // We are not at the end of the path. Recurse...
            Some(index) => match self.clone() {
                Node::Fork(mut fork) => {
                    // Choose the branch on the specified side.
                    let node = fork[index]
                        .as_ref()
                        .map(|link| link.node.clone())
                        .unwrap_or_default();

                    // Replace its value recursively.
                    let (node, new) = node.insert(rhs, leaf);
                    fork[index] = Some(Rc::new(node.into()));
                    (Node::Fork(fork), new)
                }

                _ => unreachable!(),
            },
        }
    }
}
