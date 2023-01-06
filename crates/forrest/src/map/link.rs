// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use digest::Digest;

use super::{fork::Fork, node::Node};
use crate::hash::Hash;

pub struct Link<D: Digest, K, V> {
    pub hash: Hash<D>,
    pub node: Node<D, K, V>,
}

impl<D: Digest, K, V> Clone for Link<D, K, V> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            node: self.node.clone(),
        }
    }
}

impl<D: Digest, K, V> Default for Link<D, K, V> {
    fn default() -> Self {
        let fork = Fork::default();

        Link {
            hash: fork.hash(),
            node: Node::Fork(fork),
        }
    }
}
