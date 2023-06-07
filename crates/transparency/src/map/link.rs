use std::sync::Arc;

use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::node::Node;

#[derive(Debug)]
pub struct Link<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> {
    hash: Hash<D>,
    node: Arc<Node<D, K>>,
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> Link<D, K> {
    pub fn new(node: Node<D, K>) -> Self {
        Self {
            hash: node.hash(),
            node: Arc::new(node),
        }
    }

    pub fn hash(&self) -> &Hash<D> {
        &self.hash
    }

    pub fn node(&self) -> &Node<D, K> {
        &self.node
    }
}

impl<D: SupportedDigest, K: std::fmt::Debug + VisitBytes + Clone + PartialEq> Clone for Link<D, K> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            node: self.node.clone(),
        }
    }
}
