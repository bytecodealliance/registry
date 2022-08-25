// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use core::ops::{Index, IndexMut};

use alloc::rc::Rc;
use digest::Digest;

use super::{hash::Hash, link::Link};

pub struct Fork<D: Digest, K, V>([Option<Rc<Link<D, K, V>>>; 2]);

impl<D: Digest, K, V> Fork<D, K, V> {
    pub fn hash(&self) -> Hash<D> {
        match &self.0 {
            [Some(l), Some(r)] => D::new_with_prefix(&[0b11])
                .chain_update(&*l.hash)
                .chain_update(&*r.hash),

            [Some(l), None] => D::new_with_prefix(&[0b10]).chain_update(&*l.hash),
            [None, Some(r)] => D::new_with_prefix(&[0b01]).chain_update(&*r.hash),
            [None, None] => D::new_with_prefix(&[0b00]),
        }
        .finalize()
        .into()
    }
}

impl<D: Digest, K, V> Clone for Fork<D, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<D: Digest, K, V> Default for Fork<D, K, V> {
    fn default() -> Self {
        Self([None, None])
    }
}

impl<D: Digest, K, V> Index<bool> for Fork<D, K, V> {
    type Output = Option<Rc<Link<D, K, V>>>;

    fn index(&self, index: bool) -> &Self::Output {
        &self.0[usize::from(index)]
    }
}

impl<D: Digest, K, V> IndexMut<bool> for Fork<D, K, V> {
    fn index_mut(&mut self, index: bool) -> &mut Self::Output {
        &mut self.0[usize::from(index)]
    }
}
