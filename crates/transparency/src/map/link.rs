use warg_crypto::hash::{Hash, SupportedDigest};

use super::node::Node;

pub struct Link<D: SupportedDigest> {
    hash: Hash<D>,
    node: Node<D>,
}

impl<D: SupportedDigest> Link<D> {
    pub fn new(node: Node<D>) -> Self {
        Self {
            hash: node.hash(),
            node,
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
        let node = Node::default();

        Link {
            hash: node.hash(),
            node,
        }
    }
}
