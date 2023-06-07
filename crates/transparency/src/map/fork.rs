use core::ops::{Index, IndexMut};
use std::marker::PhantomData;

use alloc::sync::Arc;

use warg_crypto::{hash::{Hash, SupportedDigest}, VisitBytes};

use super::{link::Link, map::hash_branch, path::Side};

#[derive(Debug)]
pub struct Fork<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> {
    key: PhantomData<K>,
    left: Arc<Link<D, K>>,
    right: Arc<Link<D, K>>,
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> Fork<D, K> {
    pub fn new(left: Arc<Link<D, K>>, right: Arc<Link<D, K>>) -> Self {
        Self { key: PhantomData, left, right }
    }

    pub fn hash(&self) -> Hash<D> {
        let lhs = self.left.as_ref().hash().clone();
        let rhs = self.right.as_ref().hash().clone();
        hash_branch(lhs, rhs)
    }
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> Clone for Fork<D, K> {
    fn clone(&self) -> Self {
        Self {
            key: PhantomData,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> Index<Side> for Fork<D, K> {
    type Output = Arc<Link<D, K>>;

    fn index(&self, index: Side) -> &Self::Output {
        match index {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> IndexMut<Side> for Fork<D, K> {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        match index {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }
}
