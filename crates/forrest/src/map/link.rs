// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use digest::Digest;

use super::{hash::Hash, node::Node};

pub struct Link<D: Digest, K, V> {
    pub hash: Hash<D>,
    pub node: Node<D, K, V>,
}

impl<D, K, V> From<Node<D, K, V>> for Link<D, K, V>
where
    D: Digest,
    K: AsRef<[u8]>,
    V: AsRef<[u8]>,
{
    fn from(node: Node<D, K, V>) -> Self {
        let mut digest = D::new();

        match &node {
            Node::Leaf(leaf) => {
                digest.update(&[0xff]);
                digest.update(D::digest(leaf.0.as_ref()));
                digest.update(D::digest(leaf.1.as_ref()));
            }

            Node::Fork([Some(l), Some(r)]) => {
                digest.update(&[0b11]);
                digest.update(&*l.hash);
                digest.update(&*r.hash);
            }

            Node::Fork([Some(l), None]) => {
                digest.update(&[0b10]);
                digest.update(&*l.hash);
            }

            Node::Fork([None, Some(r)]) => {
                digest.update(&[0b01]);
                digest.update(&*r.hash);
            }

            Node::Fork([None, None]) => digest.update(&[0b00]),
        }

        let hash = digest.finalize().into();
        Self { hash, node }
    }
}
