use core::ops::{Index, IndexMut};

use alloc::sync::Arc;

use warg_crypto::hash::{Hash, SupportedDigest};

use super::link::Link;

pub struct Fork<D: SupportedDigest, K, V>([Option<Arc<Link<D, K, V>>>; 2]);

impl<D: SupportedDigest, K, V> Fork<D, K, V> {
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

impl<D: SupportedDigest, K, V> Clone for Fork<D, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<D: SupportedDigest, K, V> Default for Fork<D, K, V> {
    fn default() -> Self {
        Self([None, None])
    }
}

impl<D: SupportedDigest, K, V> Index<bool> for Fork<D, K, V> {
    type Output = Option<Arc<Link<D, K, V>>>;

    fn index(&self, index: bool) -> &Self::Output {
        &self.0[usize::from(index)]
    }
}

impl<D: SupportedDigest, K, V> IndexMut<bool> for Fork<D, K, V> {
    fn index_mut(&mut self, index: bool) -> &mut Self::Output {
        &mut self.0[usize::from(index)]
    }
}
