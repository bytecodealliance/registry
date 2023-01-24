use core::fmt::Debug;
use std::marker::PhantomData;

use alloc::{vec, vec::Vec};

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::node::{Node, Side};
use super::{hash_branch, hash_empty, hash_leaf, Checkpoint, LogBuilder, LogData};

/// A verifiable log where the node hashes are stored
/// contiguously in memory by index.
#[derive(Debug, Clone)]
pub struct VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// The number of entries
    length: usize,
    /// The tree data structure
    tree: Vec<Hash<D>>,
    /// Marker for value type
    _value: PhantomData<V>,
}

/// Height is the number of child-edges between the node and leaves
/// A leaf has height 0
///
/// Length is the number of total leaf nodes present
impl<D, V> VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn get_digest(&self, node: Node) -> Hash<D> {
        self.tree[node.index()].clone()
    }

    fn set_digest(&mut self, node: Node, digest: Hash<D>) {
        self.tree[node.index()] = digest;
    }

    /// Get the root of the log when it was at some length
    fn root_at(&self, length: usize) -> Option<Hash<D>> {
        if length > self.length {
            return None;
        }

        let roots = Node::broots_for_len(length);

        let result = roots
            .into_iter()
            .rev()
            .map(|node| self.hash_for(node).unwrap())
            .reduce(|old, new| {
                // Ordering due to reversal of iterator
                hash_branch::<D>(new, old)
            })
            .unwrap_or(hash_empty::<D>());

        Some(result)
    }
}

impl<D, V> Default for VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn default() -> Self {
        VecLog {
            length: 0,
            tree: vec![],
            _value: PhantomData,
        }
    }
}

impl<D, V> LogBuilder<D, V> for VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn checkpoint(&self) -> Checkpoint<D> {
        Checkpoint {
            root: self.root_at(self.length).unwrap(),
            length: self.length,
        }
    }

    fn push(&mut self, entry: &V) -> Node {
        // Compute entry digest
        let leaf_digest = hash_leaf::<D>(entry);

        // Record entry
        self.length += 1;

        // Push spacer (if necessary) and entry digest
        if self.length != 1 {
            self.tree.push(hash_empty::<D>());
        }
        let leaf_node = Node(self.tree.len());
        self.tree.push(leaf_digest.clone());

        // Fill in newly known hashes
        let mut current_digest = leaf_digest.clone();
        let mut current_node = leaf_node;
        while current_node.side() == Side::Right {
            let sibling = current_node.left_sibling();
            let parent = current_node.parent();

            let lhs = self.get_digest(sibling);
            let rhs = current_digest;

            current_digest = hash_branch::<D>(lhs, rhs);
            current_node = parent;

            self.set_digest(current_node, current_digest.clone());
        }

        leaf_node
    }
}

impl<D, V> AsRef<[Hash<D>]> for VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn as_ref(&self) -> &[Hash<D>] {
        &self.tree[..]
    }
}

impl<D, V> LogData<D, V> for VecLog<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn hash_for(&self, node: Node) -> Option<Hash<D>> {
        self.tree.get(node.index()).map(|h| h.clone())
    }

    fn has_hash(&self, node: Node) -> bool {
        self.tree.len() > node.index()
    }
}

#[cfg(test)]
mod tests {
    use warg_crypto::hash::Sha256;

    use crate::log::proof::InclusionProofError;

    use super::*;

    fn naive_merkle<D: SupportedDigest, E: VisitBytes>(elements: &[E]) -> Hash<D> {
        let res = match elements.len() {
            0 => hash_empty::<D>(),
            1 => hash_leaf::<D>(&elements[0]),
            _ => {
                let k = elements.len().next_power_of_two() / 2;
                let left = naive_merkle::<D, E>(&elements[..k]);
                let right = naive_merkle::<D, E>(&elements[k..]);
                hash_branch::<D>(left, right)
            }
        };
        res
    }

    #[test]
    fn test_log_modifications() {
        let data = [
            "93", "67", "30", "37", "23", "75", "57", "89", "76", "42", "9", "14", "40", "59",
            "26", "66", "77", "38", "47", "34", "8", "81", "101", "102", "103",
        ];

        let mut tree: VecLog<Sha256, &str> = VecLog::default();
        let mut roots = Vec::new();

        for i in 0..data.len() {
            tree.push(&data[i]);

            let naive_root = naive_merkle::<Sha256, _>(&data[..i + 1]);

            let tree_root = tree.checkpoint().root();
            assert_eq!(
                tree_root, naive_root,
                "at {}: (in-order) {:?} != (naive) {:?}",
                i, tree_root, naive_root
            );

            roots.push(tree_root);
        }

        // Check inclusion proofs
        for (i, _) in data.iter().enumerate() {
            let leaf_node = Node(i * 2);

            for (j, root) in roots.iter().enumerate() {
                let log_length = j + 1;
                let inc_proof = tree.prove_inclusion(leaf_node, log_length);
                let result = inc_proof.evaluate_value(&tree, &data[i]);
                if j >= i {
                    assert!(result.is_ok());
                    assert_eq!(root.clone(), result.unwrap());
                } else {
                    assert!(result.is_err());
                    assert_eq!(result.unwrap_err(), InclusionProofError::LeafTooNew);
                }
            }
        }

        // Check consistency proofs
        for (i, _) in data.iter().enumerate() {
            let old_length = i + 1;
            let old_root = tree.root_at(old_length).unwrap();

            for (j, new_root) in roots.iter().enumerate().skip(i) {
                let new_root = new_root.clone();
                let new_length = j + 1;

                let proof = tree.prove_consistency(old_length, new_length);
                let results = proof.evaluate(&tree).unwrap();
                let (found_old_root, found_new_root) = results;
                assert_eq!(old_root, found_old_root);
                assert_eq!(new_root, found_new_root);
            }
        }
    }
}
