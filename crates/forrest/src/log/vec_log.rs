use core::fmt::Debug;

use alloc::{vec, vec::Vec};

use warg_crypto::hash::{SupportedDigest, Hash};

use super::node::{Node, Side};
use super::{hash_branch, hash_empty, hash_leaf, Checkpoint, HashProvider, VerifiableLog};

/// A verifiable log where the node hashes are stored
/// contiguously in memory by index.
#[derive(Debug, Clone)]
pub struct VecLog<D>
where
    D: SupportedDigest
{
    /// The number of entries
    length: usize,
    /// The tree data structure
    tree: Vec<Hash<D>>,
}

/// Height is the number of child-edges between the node and leaves
/// A leaf has height 0
///
/// Length is the number of total leaf nodes present
impl<D> VecLog<D>
where
    D: SupportedDigest
{
    fn get_digest(&self, node: Node) -> Hash<D> {
        self.tree[node.index()].clone()
    }

    fn set_digest(&mut self, node: Node, digest: Hash<D>) {
        self.tree[node.index()] = digest;
    }
}

impl<D> Default for VecLog<D>
where
    D: SupportedDigest
{
    fn default() -> Self {
        VecLog {
            length: 0,
            tree: vec![],
        }
    }
}

impl<D> VerifiableLog<D> for VecLog<D>
where
    D: SupportedDigest
{
    fn root(&self) -> Hash<D> {
        self.root_at(self.checkpoint()).unwrap()
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.length)
    }

    fn root_at(&self, point: Checkpoint) -> Option<Hash<D>> {
        if point > self.checkpoint() {
            return None;
        }

        let roots = Node::broots_for_len(point.length());

        let result = roots
            .into_iter()
            .rev()
            .map(|node| self.get_digest(node))
            .reduce(|old, new| {
                // Ordering due to reversal of iterator
                hash_branch::<D>(new, old)
            })
            .unwrap_or(hash_empty::<D>());

        Some(result)
    }

    fn hash_for(&self, node: Node) -> Option<Hash<D>> {
        self.tree.get(node.index()).map(|h| h.clone())
    }

    fn push(&mut self, entry: impl AsRef<[u8]>) -> (Checkpoint, Node) {
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

        (Checkpoint(self.length), leaf_node)
    }
}

impl<D> AsRef<[Hash<D>]> for VecLog<D>
where
    D: SupportedDigest
{
    fn as_ref(&self) -> &[Hash<D>] {
        &self.tree[..]
    }
}

impl<D> HashProvider<D> for VecLog<D>
where
    D: SupportedDigest
{
    fn hash_for(&self, node: Node) -> Option<Hash<D>> {
        VerifiableLog::hash_for(self, node)
    }

    fn has_hash(&self, node: Node) -> bool {
        VerifiableLog::hash_for(self, node).is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::dbg;

    use warg_crypto::hash::Sha256;

    use crate::log::proofs::{ConsistencyProof, InclusionProof, InclusionProofError};

    use super::*;

    fn naive_merkle<D: SupportedDigest, E: AsRef<[u8]>>(elements: &[E]) -> Hash<D> {
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
        let data: [&str; 25] = [
            "93", "67", "30", "37", "23", "75", "57", "89", "76", "42", "9", "14", "40", "59",
            "26", "66", "77", "38", "47", "34", "8", "81", "101", "102", "103",
        ];

        let mut tree: VecLog<Sha256> = VecLog::default();
        let mut roots = Vec::new();

        for i in 0..data.len() {
            tree.push(data[i]);

            let naive_root = naive_merkle::<Sha256, _>(&data[..i + 1]);

            let tree_root = tree.root();
            assert_eq!(
                tree_root, naive_root,
                "at {}: (in-order) {:?} != (naive) {:?}",
                i, tree_root, naive_root
            );

            roots.push(tree_root);
        }

        // Check inclusion proofs
        for (i, _entry) in data.iter().enumerate() {
            let leaf_node = Node(i * 2);

            for (j, root) in roots.iter().enumerate() {
                let now = Checkpoint(j + 1);
                let inc_proof = InclusionProof {
                    point: now,
                    leaf: leaf_node,
                };
                let result = inc_proof.evaluate(&tree);
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
            let old_point = Checkpoint(i + 1);

            for (j, new_root) in roots.iter().enumerate().skip(i) {
                let new_point = Checkpoint(j + 1);

                let con_proof = ConsistencyProof {
                    old_point,
                    new_point,
                };
                let mut old_broots = Vec::new();

                for inc_proof in con_proof.inclusions().unwrap() {
                    old_broots.push(inc_proof.leaf.clone());
                    let result = inc_proof.evaluate(&tree);
                    dbg!(i, j, inc_proof);
                    dbg!(&result);
                    assert!(result.is_ok());
                    assert_eq!(new_root.clone(), result.unwrap());
                }
            }
        }
    }
}
