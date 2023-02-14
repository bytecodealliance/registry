use std::marker::PhantomData;

use alloc::vec::Vec;
use thiserror::Error;
use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::{hash_branch, hash_leaf, node::Node, LogData};

/// A proof that a leaf is present for a root
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionProof<D: SupportedDigest, V: VisitBytes> {
    /// The node that you are checking is present in the given point.
    leaf: Node,
    /// The point in the logs history where the leaf should be present
    log_length: usize,
    /// Marker for digest type
    _digest: PhantomData<D>,
    /// Marker for value type
    _value: PhantomData<V>,
}

/// An error occurring when attempting to validate an inclusion proof.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InclusionProofError {
    /// Indicates that the leaf is too new to be present at
    /// the given point in the log history.
    #[error("Leaf newer than when it should be included")]
    LeafTooNew,
    /// Indicates that certain hashes weren't known that are
    /// needed to perform proof validation.
    #[error("Required hash for proof is not available")]
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
    pub(crate) nodes: Vec<Node>,
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

impl<D, V> InclusionProof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    pub(crate) fn new(leaf: Node, log_length: usize) -> Self {
        Self {
            leaf,
            log_length,
            _digest: PhantomData,
            _value: PhantomData,
        }
    }

    /// Get the node that this proof proves the inclusion of
    pub fn leaf(&self) -> Node {
        self.leaf
    }

    /// Get the length of the log this proof shows the leaf was included in
    pub fn log_length(&self) -> usize {
        self.log_length
    }

    /// Collects all of the node indices that must be visited
    /// in order to validate the inlcusion proof into.
    pub fn walk(&self) -> Result<InclusionProofWalk, InclusionProofError> {
        let length = self.log_length;
        let broots = Node::broots_for_len(length);
        let mut current_node = self.leaf;

        if !current_node.exists_at_length(length) {
            return Err(InclusionProofError::LeafTooNew);
        }

        let mut nodes = Vec::new();

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
    pub fn evaluate_value(
        &self,
        hashes: &impl LogData<D, V>,
        value: &V,
    ) -> Result<Hash<D>, InclusionProofError> {
        self.evaluate_hash(hashes, hash_leaf(value))
    }

    /// Evaluate an inclusion proof.
    /// Callers should verify that the returned root matches their expectation.
    ///
    /// Walks the inclusion proof, hashes each layer, returns the root hash.
    pub fn evaluate_hash(
        &self,
        hashes: &impl LogData<D, V>,
        hash: Hash<D>,
    ) -> Result<Hash<D>, InclusionProofError> {
        let leaf = (self.leaf, hash);
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
            .fold(leaf, combine);

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
pub struct ConsistencyProof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// The older of the two points
    pub old_length: usize,
    /// The newer of the two points
    pub new_length: usize,
    /// Marker for digest type
    _digest: PhantomData<D>,
    /// Marker for value type
    _value: PhantomData<V>,
}

/// Errors occurring when validating a consistency proof
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsistencyProofError {
    /// Happens when old_length > new_length
    #[error("Tries to prove later value comes before earlier")]
    PointsOutOfOrder,
    /// Happens when hashes required for evaluation were not present
    #[error("A hash needed for evaluation was not available")]
    HashNotKnown,
    /// Happens when an inclusion proof is evaluated and has an error
    #[error("Constituent inclusion proof failed")]
    InclusionError(#[from] InclusionProofError),
    /// Happens when two inclusion proofs are evaluated and produce different roots
    #[error("Constituent inclusion proofs diverge produce different roots")]
    DivergingRoots,
}

impl<D, V> ConsistencyProof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    pub(crate) fn new(old_length: usize, new_length: usize) -> Self {
        Self {
            old_length,
            new_length,
            _digest: PhantomData,
            _value: PhantomData,
        }
    }

    /// Evaluate an inclusion proof.
    /// Callers should verify that the returned root matches their expectation.
    ///
    /// Walks the inclusion proof, hashes each layer, returns the root hash.
    pub fn evaluate(
        &self,
        hashes: &impl LogData<D, V>,
    ) -> Result<(Hash<D>, Hash<D>), ConsistencyProofError> {
        let mut old_broots = Vec::new();
        let mut new_root = None;

        for inc_proof in self.inclusions().unwrap() {
            let leaf_hash = hashes
                .hash_for(inc_proof.leaf())
                .ok_or(ConsistencyProofError::HashNotKnown)?;
            old_broots.push(leaf_hash.clone());
            let found_root = inc_proof.evaluate_hash(hashes, leaf_hash)?;
            if let Some(previous_root) = &new_root {
                if previous_root != &found_root {
                    return Err(ConsistencyProofError::DivergingRoots);
                }
            } else {
                new_root = Some(found_root);
            }
        }

        let old_root = old_broots
            .into_iter()
            .rev()
            .reduce(|new, old| hash_branch(old, new));
        // Unwrap is safe because the minimal consistency proof always has at least one inclusion proof
        let old_root = old_root.unwrap();
        let new_root = new_root.unwrap();
        Ok((old_root, new_root))
    }

    /// Convert the consistency proof into a sequence of inclusion proofs.
    /// Each inclusion proof verifies that one of the balanced roots
    /// of the old tree is present in the root of the new tree.
    pub fn inclusions(&self) -> Result<Vec<InclusionProof<D, V>>, ConsistencyProofError> {
        if self.old_length > self.new_length {
            return Err(ConsistencyProofError::PointsOutOfOrder);
        }

        let inclusions = Node::broots_for_len(self.old_length)
            .into_iter()
            .map(|broot| InclusionProof::new(broot, self.new_length))
            .collect();

        Ok(inclusions)
    }
}

#[cfg(test)]
mod tests {
    use crate::log::{LogBuilder, VecLog};

    use super::*;

    use warg_crypto::hash::Sha256;

    #[test]
    fn test_inc_even_2() {
        let mut log: VecLog<Sha256, u8> = VecLog::default();

        log.push(&100);
        log.push(&102);

        let inc_proof = InclusionProof::new(Node(0), 2);
        let expected = InclusionProofWalk {
            nodes: vec![Node(2)],
            initial_walk_len: 1,
            lower_broots: 0,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);

        assert_eq!(
            inc_proof.evaluate_value(&log, &100).unwrap(),
            log.as_ref()[1].clone()
        );
    }

    #[test]
    fn test_inc_odd_3() {
        let mut log: VecLog<Sha256, u8> = VecLog::default();

        log.push(&100);
        log.push(&102);
        log.push(&104);

        let root: Hash<Sha256> = hash_branch(log.as_ref()[1].clone(), log.as_ref()[4].clone());

        // node 0
        let inc_proof = InclusionProof::new(Node(0), 3);
        let expected = InclusionProofWalk {
            nodes: vec![Node(2), Node(4)],
            initial_walk_len: 1,
            lower_broots: 1,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate_value(&log, &100).unwrap(), root);

        // node 2
        let inc_proof = InclusionProof::new(Node(2), 3);
        let expected = InclusionProofWalk {
            nodes: vec![Node(0), Node(4)],
            initial_walk_len: 1,
            lower_broots: 1,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate_value(&log, &102u8).unwrap(), root);

        // node 4
        let inc_proof = InclusionProof::new(Node(4), 3);
        let expected = InclusionProofWalk {
            nodes: vec![Node(1)],
            initial_walk_len: 0,
            lower_broots: 0,
            upper_broots: 1,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate_value(&log, &104u8).unwrap(), root);
    }

    #[test]
    fn test_inc_odd_7() {
        let mut log: VecLog<Sha256, u8> = VecLog::default();

        log.push(&100);
        log.push(&102);
        log.push(&104);
        log.push(&106);
        log.push(&108);
        log.push(&110);
        log.push(&112);

        let artificial_branch: Hash<Sha256> =
            hash_branch(log.as_ref()[9].clone(), log.as_ref()[12].clone());
        let root: Hash<Sha256> = hash_branch(log.as_ref()[3].clone(), artificial_branch);

        // node 6
        let inc_proof = InclusionProof::new(Node(6), 7);
        let expected = InclusionProofWalk {
            nodes: vec![Node(4), Node(1), Node(12), Node(9)],
            initial_walk_len: 2,
            lower_broots: 2,
            upper_broots: 0,
        };
        assert_eq!(inc_proof.walk().unwrap(), expected);
        assert_eq!(inc_proof.evaluate_value(&log, &106).unwrap(), root);
    }
}
