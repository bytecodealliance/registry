// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use warg_crypto::hash::{Hash, SupportedDigest};

use super::{fork::Fork, node::Node};

pub struct Link<D: SupportedDigest, K, V> {
    pub hash: Hash<D>,
    pub node: Node<D, K, V>,
}

impl<D: SupportedDigest, K, V> Clone for Link<D, K, V> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            node: self.node.clone(),
        }
    }
}

impl<D: SupportedDigest, K, V> Default for Link<D, K, V> {
    fn default() -> Self {
        let fork = Fork::default();

        Link {
            hash: fork.hash(),
            node: Node::Fork(fork),
        }
    }
}
