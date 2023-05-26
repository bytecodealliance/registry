use std::sync::Arc;

use warg_crypto::hash::{Hash, SupportedDigest};

use super::node::Node;

pub struct Link<D: SupportedDigest> {
    hash: Hash<D>,
    node: Arc<Node<D>>,
}

impl<D: SupportedDigest> Link<D> {
    pub fn new(node: Node<D>) -> Self {
        Self {
            hash: node.hash(),
            node: Arc::new(node),
        }
    }

    pub fn hash(&self) -> &Hash<D> {
        &self.hash
    }

    pub fn node(&self) -> &Node<D> {
        &self.node
    }
}

impl<D: SupportedDigest> Clone for Link<D> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash.clone(),
            node: self.node.clone(),
        }
    }
}

impl<D: SupportedDigest> Default for Link<D> {
    fn default() -> Self {

        Link {
            hash: Node::Empty(0).hash(),
            node: Arc::new(Node::Empty(0)),
        }
    }
}
