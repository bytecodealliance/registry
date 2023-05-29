use core::ops::{Index, IndexMut};

use alloc::sync::Arc;

use warg_crypto::hash::{Hash, SupportedDigest};

use super::{link::Link, map::hash_branch, path::Side};

pub struct Fork<D: SupportedDigest> {
    pub left: Arc<Link<D>>,
    pub right: Arc<Link<D>>,
}

impl<D: SupportedDigest> Fork<D> {
    pub fn new(left: Arc<Link<D>>, right: Arc<Link<D>>) -> Self {
        Self { left, right }
    }

    pub fn hash(&self) -> Hash<D> {
        let lhs = self.left.hash();
        let rhs = self.right.hash();
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

impl<D: SupportedDigest> Index<Side> for Fork<D> {
    type Output = Arc<Link<D>>;

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
