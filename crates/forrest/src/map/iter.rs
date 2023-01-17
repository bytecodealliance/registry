// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use alloc::{vec, vec::Vec};
use core::iter::FusedIterator;

use warg_crypto::hash::SupportedDigest;

use super::node::Node;

/// An iterator over map items
pub struct Iter<'a, D: SupportedDigest, K, V> {
    stack: Vec<(&'a Node<D, K, V>, bool)>,
    total: usize,
    index: usize,
}

impl<'a, D: SupportedDigest, K, V> Iter<'a, D, K, V> {
    pub(crate) fn new(root: &'a Node<D, K, V>, total: usize) -> Self {
        Self {
            stack: vec![(root, false)],
            total,
            index: 0,
        }
    }
}

impl<'a, D: SupportedDigest, K, V> ExactSizeIterator for Iter<'a, D, K, V> {}
impl<'a, D: SupportedDigest, K, V> FusedIterator for Iter<'a, D, K, V> {}

impl<'a, D: SupportedDigest, K, V> Iterator for Iter<'a, D, K, V> {
    type Item = (&'a K, &'a V);

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.total - self.index;
        (size, Some(size))
    }

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop() {
            None => None,

            Some((node, idx)) => match node {
                Node::Leaf(leaf) => {
                    self.index += 1;
                    Some((&leaf.0, &leaf.1))
                }

                Node::Fork(fork) => {
                    if !idx {
                        self.stack.push((node, true));
                    }

                    if let Some(link) = fork[idx].as_deref() {
                        self.stack.push((&link.node, false));
                    }

                    self.next()
                }
            },
        }
    }
}
