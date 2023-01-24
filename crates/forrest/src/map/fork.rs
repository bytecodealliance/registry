use core::ops::{Index, IndexMut};

use alloc::sync::Arc;

use warg_crypto::hash::{Hash, SupportedDigest};

use super::{link::Link, map::hash_branch, path::Side};

pub struct Fork<D: SupportedDigest> {
    left: Option<Arc<Link<D>>>,
    right: Option<Arc<Link<D>>>,
}

impl<D: SupportedDigest> Fork<D> {
    pub fn hash(&self) -> Hash<D> {
        let lhs = self.left.as_ref().map(|left| left.hash().clone());
        let rhs = self.right.as_ref().map(|right| right.hash().clone());
        hash_branch(lhs, rhs)
    }
}

impl<D: SupportedDigest> Clone for Fork<D> {
    fn clone(&self) -> Self {
        Self {
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<D: SupportedDigest> Default for Fork<D> {
    fn default() -> Self {
        Self {
            left: None,
            right: None,
        }
    }
}

impl<D: SupportedDigest> Index<Side> for Fork<D> {
    type Output = Option<Arc<Link<D>>>;

    fn index(&self, index: Side) -> &Self::Output {
        match index {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }
}

impl<D: SupportedDigest> IndexMut<Side> for Fork<D> {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        match index {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }
}
