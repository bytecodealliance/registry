use alloc::vec::Vec;
use warg_crypto::hash::{Hash, SupportedDigest};

use super::{hash_branch, node::Node, Checkpoint, HashProvider};

/// A proof that a leaf is present for a root
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionProof {
    /// The point in the logs history where the leaf should be present
    pub point: Checkpoint,
    /// The node that you are checking is present in the given point.
    pub leaf: Node,
}

/// An error occuring when attempting to validate an inclusion proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InclusionProofError {
    /// Indicates that the leaf is too new to be present at
    /// the given point in the log history.
    LeafTooNew,
    /// Indicates that certain hashes weren't known that are
    /// needed to perform proof validation.
    HashNotKnown,
}

/// The nodes visited when verifying the inclusion proof.
///
/// The first [InclusionProofWalk.initial_walk_len] nodes
/// describe the walk up to the balanced root which is
/// the leafs ancestor.
///
/// The next [InclusionProofWalk.lower_broots] nodes
/// describe the walk from the rightmost (lowest) broot
/// up towards the broot that was reached.
///
/// The next [InclusionProofWalk.upper_broots] nodes
/// describes the walk from the intersection of the
/// previous two to the leftmost (tallest) broot.
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
    /// Collects all of the node indices that must be visited
    /// in order to validate the inlcusion proof into.
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

    /// Evaluate an inclusion proof.
    /// Callers should verify that the returned root matches their expectation.
    ///
    /// Walks the inclusion proof, hashes each layer, returns the root hash.
    pub fn evaluate<D: SupportedDigest>(
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

fn combine<D: SupportedDigest>(first: (Node, Hash<D>), second: (Node, Hash<D>)) -> (Node, Hash<D>) {
    let (lhs, rhs) = if first.0.index() < second.0.index() {
        (first.1, second.1)
    } else {
        (second.1, first.1)
    };

    (second.0, hash_branch::<D>(lhs, rhs))
}

/// A proof of the consistency between two points in the
/// logs history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsistencyProof {
    /// The older of the two points
    pub old_point: Checkpoint,
    /// The newer of the two points
    pub new_point: Checkpoint,
}

/// Errors occuring when validating a consistency proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsistencyProofError {
    /// Indicates that the new point is actually older
    PointsOutOfOrder,
}

impl ConsistencyProof {
    /// Convert the consistency proof into a sequence of inclusion proofs.
    /// Each inclusion proof verifies that one of the balanced roots
    /// of the old tree is present in the root of the new tree.
    pub fn inclusions(&self) -> Result<Vec<InclusionProof>, ConsistencyProofError> {
        if self.old_point > self.new_point {
            return Err(ConsistencyProofError::PointsOutOfOrder);
        }

        let incls = Node::broots_for_len(self.old_point.length())
            .into_iter()
            .map(|broot| InclusionProof {
                point: self.new_point,
                leaf: broot,
            })
            .collect();

        Ok(incls)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::log::{VecLog, VerifiableLog};

    use super::*;

    use warg_crypto::hash::Sha256;

    impl<D: SupportedDigest> HashProvider<D> for Vec<Hash<D>> {
        fn hash_for(&self, node: Node) -> Option<Hash<D>> {
            Some(self.get(node.index())?.clone())
        }

        fn has_hash(&self, node: Node) -> bool {
            self.get(node.index()).is_some()
        }
    }

    #[test]
    fn test_inc_even_2() {
        let mut log: VecLog<Sha256> = VecLog::default();

        log.push(&[100u8]);
        log.push(&[102u8]);

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

        assert_eq!(inc_proof.evaluate(&log).unwrap(), log.as_ref()[1].clone());
    }

    #[test]
    fn test_inc_odd_3() {
        let mut log: VecLog<Sha256> = VecLog::default();

        log.push(&[100u8]);
        log.push(&[102u8]);
        log.push(&[104u8]);

        let root: Hash<Sha256> = hash_branch(log.as_ref()[1].clone(), log.as_ref()[4].clone());

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
        assert_eq!(inc_proof.evaluate(&log).unwrap(), root);

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
        assert_eq!(inc_proof.evaluate(&log).unwrap(), root);

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
        assert_eq!(inc_proof.evaluate(&log).unwrap(), root);
    }

    #[test]
    fn test_inc_odd_7() {
        let mut log: VecLog<Sha256> = VecLog::default();

        log.push(&[100u8]);
        log.push(&[102u8]);
        log.push(&[104u8]);
        log.push(&[106u8]);
        log.push(&[108u8]);
        log.push(&[110u8]);
        log.push(&[112u8]);

        let artificial_branch: Hash<Sha256> =
            hash_branch(log.as_ref()[9].clone(), log.as_ref()[12].clone());
        let root: Hash<Sha256> = hash_branch(log.as_ref()[3].clone(), artificial_branch);

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
        assert_eq!(inc_proof.evaluate(&log).unwrap(), root);
    }
}
