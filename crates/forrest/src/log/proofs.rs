use alloc::vec::Vec;

use super::super::map::hash::Hash;
use digest::Digest;

use super::{hash_branch, node::Node, Checkpoint, HashProvider};

/// A proof that a leaf is present for a root
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionProof {
    pub point: Checkpoint,
    pub leaf: Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InclusionProofError {
    LeafTooNew,
    HashNotKnown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProofWalk {
    nodes: Vec<Node>,
    initial_walk_len: usize,
    lower_broots: usize,
    upper_broots: usize,
}

impl InclusionProofWalk {
    fn initial_walk_end(&self) -> usize {
        self.initial_walk_len
    }

    fn lower_broot_walk_end(&self) -> usize {
        self.initial_walk_end() + self.lower_broots
    }

    fn upper_broot_walk_end(&self) -> usize {
        self.lower_broot_walk_end() + self.upper_broots
    }

    fn initial_walk(&self) -> &[Node] {
        &self.nodes[..self.initial_walk_end()]
    }

    fn lower_broot_walk(&self) -> &[Node] {
        &self.nodes[self.initial_walk_end()..self.lower_broot_walk_end()]
    }

    fn upper_broot_walk(&self) -> &[Node] {
        &self.nodes[self.lower_broot_walk_end()..self.upper_broot_walk_end()]
    }
}

impl InclusionProof {
    pub fn walk(&self) -> Result<InclusionProofWalk, InclusionProofError> {
        let length = self.point.length();
        let broots = Node::broots_for_len(length);
        let mut current_node = self.leaf;

        if !current_node.exists_at_length(length) {
            return Err(InclusionProofError::LeafTooNew);
        }

        let mut nodes = Vec::new();
        nodes.push(self.leaf);

        // Walk upwards until you hit a balanced root for the original tree
        while !broots.contains(&current_node) {
            let sibling = current_node.sibling();
            nodes.push(sibling);
            current_node = current_node.parent();
        }

        let initial_walk_len = nodes.len();

        let index = broots
            .iter()
            .position(|broot| *broot == current_node)
            .unwrap();

        let lower_broots = broots.len() - index - 1;
        for broot in broots[index + 1..].iter().rev() {
            nodes.push(*broot);
        }

        let upper_broots = index;
        for broot in broots[..index].iter().rev() {
            nodes.push(*broot);
        }

        Ok(InclusionProofWalk {
            nodes,
            initial_walk_len,
            lower_broots,
            upper_broots,
        })
    }

    pub fn evaluate<D: Digest>(
        &self,
        hashes: &impl HashProvider<D>,
    ) -> Result<Hash<D>, InclusionProofError> {
        let walk = self.walk()?;

        // Ensure all nodes are known
        if walk.nodes.iter().any(|node| !hashes.has_hash(*node)) {
            return Err(InclusionProofError::HashNotKnown);
        }

        // Perform initial walk up to the ancestor broot
        let current = walk
            .initial_walk()
            .iter()
            .map(|node| (*node, hashes.hash_for(*node).unwrap()))
            .reduce(combine)
            .unwrap();

        // Summarize all of the smaller broots
        let lower_broot = walk
            .lower_broot_walk()
            .iter()
            .map(|node| (*node, hashes.hash_for(*node).unwrap()))
            .reduce(combine);

        // Combine broot with summary of smaller roots
        let current = match lower_broot {
            Some(lower_broot) => combine(current, lower_broot),
            None => current,
        };

        // Combine with any larger roots
        let current = walk
            .upper_broot_walk()
            .iter()
            .map(|node| (*node, hashes.hash_for(*node).unwrap()))
            .fold(current, combine);
    
        Ok(current.1)
    }
}

fn combine<D: Digest>(first: (Node, Hash<D>), second: (Node, Hash<D>)) -> (Node, Hash<D>) {
    let (lhs, rhs) = if first.0.index() < second.0.index() {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };

    (second.0, hash_branch::<D>(lhs, rhs))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsistencyProof {
    pub old_point: Checkpoint,
    pub new_point: Checkpoint,
}

impl ConsistencyProof {
    pub fn inclusions(&self) -> Vec<InclusionProof> {
        Node::broots_for_len(self.old_point.length())
            .into_iter()
            .map(|broot| InclusionProof {
                point: self.new_point,
                leaf: broot,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::log::{hash_empty, hash_leaf};

    use super::*;

    use super::super::super::map::hash::Hash;
    use sha2::Sha256;

    impl<D: Digest> HashProvider<D> for Vec<Hash<D>> {
        fn hash_for(&self, node: Node) -> Option<Hash<D>> {
            Some(self.get(node.index())?.clone())
        }

        fn has_hash(&self, node: Node) -> bool {
            self.get(node.index()).is_some()
        }
    }

    #[test]
    fn test_inc_even_2() {
        let leaf_0 = hash_leaf(&[100u8]);
        let leaf_2 = hash_leaf(&[102u8]);
        let branch_1 = hash_branch(leaf_0.clone(), leaf_2.clone());

        let data: Vec<Hash<Sha256>> = vec![leaf_0, branch_1, leaf_2];

        let inc_proof = InclusionProof {
            point: Checkpoint(2),
            leaf: Node(0),
        };
        let expected = InclusionProofWalk {
            nodes: vec![Node(0), Node(2)],
            initial_walk_len: 2,
            lower_broots: 0,
            upper_broots: 0,
            
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);

        assert_eq!(inc_proof.evaluate(&data).unwrap(), data[1].clone());
    }

    #[test]
    fn test_inc_odd_3() {
        let leaf_0 = hash_leaf(&[100u8]);
        let leaf_2 = hash_leaf(&[102u8]);
        let branch_1 = hash_branch(leaf_0.clone(), leaf_2.clone());
        let leaf_4 = hash_leaf(&[104u8]);
        let branch_3 = hash_empty();
        let root: Hash<Sha256> = hash_branch(branch_1.clone(), leaf_4.clone());

        let data: Vec<Hash<Sha256>> = vec![leaf_0, branch_1, leaf_2, branch_3, leaf_4];

        // node 0
        let inc_proof = InclusionProof {
            point: Checkpoint(3),
            leaf: Node(0),
        };
        let expected = InclusionProofWalk {
            nodes: vec![Node(0), Node(2), Node(4)],
            initial_walk_len: 2,
            lower_broots: 1,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate(&data).unwrap(), root);

        // node 2
        let inc_proof = InclusionProof {
            point: Checkpoint(3),
            leaf: Node(2),
        };
        let expected = InclusionProofWalk {
            nodes: vec![Node(2), Node(0), Node(4)],
            initial_walk_len: 2,
            lower_broots: 1,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate(&data).unwrap(), root);

        // node 4
        let inc_proof = InclusionProof {
            point: Checkpoint(3),
            leaf: Node(4),
        };
        let expected = InclusionProofWalk {
            nodes: vec![Node(4), Node(1)],
            initial_walk_len: 1,
            lower_broots: 0,
            upper_broots: 1,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate(&data).unwrap(), root);
    }

    #[test]
    fn test_inc_odd_7() {
        let leaf_0 = hash_leaf(&[100u8]);
        let leaf_2 = hash_leaf(&[102u8]);
        let branch_1 = hash_branch(leaf_0.clone(), leaf_2.clone());

        let leaf_4 = hash_leaf(&[104u8]);
        let leaf_6 = hash_leaf(&[106u8]);
        let branch_5 = hash_branch(leaf_4.clone(), leaf_6.clone());

        let branch_3 = hash_branch(branch_1.clone(), branch_5.clone());

        let leaf_8 = hash_leaf(&[108u8]);
        let leaf_10 = hash_leaf(&[110u8]);
        let branch_9 = hash_branch(leaf_8.clone(), leaf_10.clone());

        let leaf_12 = hash_leaf(&[112u8]);

        let branch_7 = hash_empty();
        let branch_11 = hash_empty();

        let artificial_branch: Hash<Sha256> = hash_branch(branch_9.clone(), leaf_12.clone());
        let root: Hash<Sha256> = hash_branch(branch_3.clone(), artificial_branch);

        let data: Vec<Hash<Sha256>> = vec![
            leaf_0, branch_1, leaf_2, branch_3, leaf_4, branch_5, leaf_6, branch_7, leaf_8,
            branch_9, leaf_10, branch_11, leaf_12,
        ];

        // node 6
        let inc_proof = InclusionProof {
            point: Checkpoint(7),
            leaf: Node(6),
        };
        let expected = InclusionProofWalk {
            nodes: vec![Node(6), Node(4), Node(1), Node(12), Node(9)],
            initial_walk_len: 3,
            lower_broots: 2,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate(&data).unwrap(), root);
    }
}
