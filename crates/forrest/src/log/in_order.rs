use alloc::boxed::Box;
use alloc::vec::Vec;
use hashbrown::HashMap;

use digest::{Digest, Output};

use super::node::{Node, Side};
use super::proofs::{ConsistencyProof, InclusionProof};
use super::VerifiableLog;

/// A data-structure for building up [merkle tree][0] logs based on [DAT][1].
/// The merkle tree computation is conformant to [RFC 6962 - Certificate Transparency][2].
///
/// It represents its data using binary in-order interval numbering.
/// This means that all of the leaf and balanced branch nodes of the tree
/// are stored in one contiguous array using a particular indexing scheme.
///
/// ## Example
/// ```text
/// 0 X \
/// 1    X
/// 2 X / \
/// 3      X
/// 4 X \ /
/// 5    X
/// 6 X /
/// ```
///
/// ## Properties
/// This has various convenient properties for traversing the structure.
/// * The height of a node is the number of trailing ones in its index.
/// * For the above reason, leaves always have even indices.
/// * The side (left/right) of a node can be computed from its index.
/// * The distance between parent/child indices is a simple function of height.
///
/// [0]: https://en.wikipedia.org/wiki/Merkle_tree
/// [1]: https://www.researchgate.net/publication/326120012_Dat_-_Distributed_Dataset_Synchronization_And_Versioning
/// [2]: https://www.rfc-editor.org/rfc/rfc6962
#[derive(Debug, Clone)]
pub struct InOrderLog<D>
where
    D: Digest,
{
    /// The number of entries
    length: usize,
    /// The tree data structure
    tree: Vec<Output<D>>,

    /// Caches the number of elements in the log at given roots
    root_cache: HashMap<Output<D>, usize>,
    /// Caches the index where a given leaf hash is found
    leaf_cache: HashMap<Output<D>, Node>,
}

/// Height is the number of child-edges between the node and leaves
/// A leaf has height 0
///
/// Length is the number of total leaf nodes present
impl<D> InOrderLog<D>
where
    D: Digest,
{
    /// Compute the balanced roots for a log with a given
    /// log length in number of leaves.
    #[inline]
    fn broots_for_len(length: usize) -> Vec<Node> {
        let mut value = length;
        let mut broot_heights = Vec::new();
        for i in 0..usize::BITS {
            let present = (value & 1) == 1;
            if present {
                broot_heights.push(i);
            }

            value = value >> 1;
        }

        let mut broots = Vec::new();
        let mut current: Option<Node> = None;
        for broot_height in broot_heights.into_iter().rev() {
            let next = match current {
                None => Node::first_node_with_height(broot_height),
                Some(last) => last.next_node_with_height(broot_height),
            };
            broots.push(next);
            current = Some(next);
        }

        broots
    }

    fn get_digest(&self, node: Node) -> Output<D> {
        self.tree[node.index()].clone()
    }

    fn set_digest(&mut self, node: Node, digest: Output<D>) {
        self.tree[node.index()] = digest;
    }

    fn consistency_proof_node(&self, node: Node, old_length: usize) -> ConsistencyProof<D> {
        if node.exists_at_length(old_length) {
            ConsistencyProof::OldRoot(self.get_digest(node))
        } else if node.has_children_at_length(old_length) {
            let (left_index, right_index) = node.children();
            let left = Box::new(self.consistency_proof_node(left_index, old_length));
            let right = Box::new(self.consistency_proof_node(right_index, old_length));
            ConsistencyProof::Hybrid { left, right }
        } else {
            ConsistencyProof::NewHash(self.get_digest(node))
        }
    }
}

impl<D> Default for InOrderLog<D>
where
    D: Digest,
{
    fn default() -> Self {
        InOrderLog {
            length: 0,
            tree: vec![],
            root_cache: HashMap::new(),
            leaf_cache: HashMap::new(),
        }
    }
}

impl<D> VerifiableLog<D> for InOrderLog<D>
where
    D: Digest,
{
    fn root(&self) -> Output<D> {
        let roots = Self::broots_for_len(self.length);

        roots
            .into_iter()
            .rev()
            .map(|node| self.get_digest(node))
            .reduce(|lhs, rhs| {
                let mut digest = D::new();
                digest.update(&[1u8]);
                digest.update(&rhs);
                digest.update(&lhs);
                digest.finalize()
            })
            .unwrap_or(D::new().finalize())
    }

    fn push(&mut self, entry: impl AsRef<[u8]>) {
        // Compute entry digest
        let mut digest = D::new();
        digest.update(&[0u8]);
        digest.update(&entry);
        let leaf_digest = digest.finalize();

        // Record entry
        self.length += 1;

        // Push spacer (if necessary) and entry digest
        if self.length != 1 {
            self.tree.push(Output::<D>::default());
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

            let mut new_digest = D::new();
            new_digest.update(&[1u8]);
            new_digest.update(lhs);
            new_digest.update(&rhs);

            current_digest = new_digest.finalize();
            current_node = parent;

            self.set_digest(current_node, current_digest.clone());
        }

        // Cache index of leaf node
        if !self.leaf_cache.contains_key(&leaf_digest) {
            // Only add to cache if not added before.
            // Storing the oldest instance of a leaf is the most robust
            // because it allows us to prove inclusion in older roots.
            self.leaf_cache.insert(leaf_digest, leaf_node);
        }
        // Cache length of log for new root
        self.root_cache.insert(self.root(), self.length);
    }

    fn prove_inclusion(&self, root: Output<D>, leaf: Output<D>) -> Option<InclusionProof<D>> {
        let length = *self.root_cache.get(&root)?;
        let balanced_roots = Self::broots_for_len(length);

        let mut path = Vec::new();
        let mut current_node = *self.leaf_cache.get(&leaf)?;

        // Walk upwards until you hit a balanced root for the original tree
        while !balanced_roots.contains(&current_node) {
            let sibling = current_node.sibling();
            path.push((sibling.side(), self.get_digest(sibling)));
            current_node = current_node.parent();
        }

        // Walk through any balanced roots to the right of the one hit
        // and compute the hash that summarizes that side of the tree.
        let right_side_root = balanced_roots
            .iter()
            .map(|broot| *broot)
            .rev()
            .take_while(|broot| *broot != current_node)
            .map(|broot| self.get_digest(broot))
            .reduce(|rhs, lhs| {
                let mut digest = D::new();
                digest.update(&[1u8]);
                digest.update(lhs);
                digest.update(rhs);
                digest.finalize()
            });

        if let Some(right_side_root) = right_side_root {
            path.push((Side::Right, right_side_root));
        }

        // Walk through any balanced roots to the left of the one hit
        for broot in balanced_roots
            .iter()
            .map(|broot| *broot)
            .take_while(|broot| *broot != current_node)
            .collect::<Vec<Node>>()
            .iter()
            .rev()
            .map(|broot| self.get_digest(*broot))
        {
            path.push((Side::Left, broot));
        }

        Some(InclusionProof { leaf, path })
    }

    fn prove_consistency(
        &self,
        old_root: Output<D>,
        new_root: Output<D>,
    ) -> Option<ConsistencyProof<D>> {
        let old_length = *self.root_cache.get(&old_root)?;
        let new_length = *self.root_cache.get(&new_root)?;

        // A log cannot be a consistent subset of a log that is shorter than it
        if old_length > new_length {
            return None;
        }

        Self::broots_for_len(new_length)
            .into_iter()
            .rev()
            .map(|index| self.consistency_proof_node(index, old_length))
            .reduce(|rhs, lhs| ConsistencyProof::Hybrid {
                left: Box::new(lhs),
                right: Box::new(rhs),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Sha256;

    use crate::log::proofs::{ConsistencyProofOutput, InclusionProofOutput};

    fn naive_merkle<D: Digest, E: AsRef<[u8]>>(elements: &[E]) -> Output<D> {
        let res = match elements.len() {
            0 => D::new().finalize(),
            1 => {
                let mut digest = D::new();
                digest.update(&[0u8]);
                digest.update(&elements[0]);
                digest.finalize()
            }
            _ => {
                let k = elements.len().next_power_of_two() / 2;
                let mut digest = D::new();
                digest.update(&[1u8]);
                digest.update(naive_merkle::<D, E>(&elements[..k]));
                digest.update(naive_merkle::<D, E>(&elements[k..]));
                digest.finalize()
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

        // let data: Vec<[u8; 4]> = (0..1_000_000u32).map(|i| i.to_le_bytes()).collect();

        let mut tree: InOrderLog<Sha256> = InOrderLog::default();
        let mut roots = Vec::new();

        for i in 0..data.len() {
            tree.push(data[i]);

            let naive_root = naive_merkle::<Sha256, &str>(&data[..i + 1]);

            let tree_root = tree.root();
            assert_eq!(
                tree_root, naive_root,
                "at {}: (in-order) {:?} != (naive) {:?}",
                i, tree_root, naive_root
            );

            roots.push(tree_root);
        }

        for (i, entry) in data.iter().enumerate() {
            // Compute leaf hash
            let mut digest = Sha256::new();
            digest.update(&[0u8]);
            digest.update(&entry);
            let leaf = digest.finalize();

            // Compute root hash
            let left_root = roots[i].clone();

            for root in roots[i..].iter() {
                // Check inclusion proofs
                let inc_proof = tree.prove_inclusion(root.clone(), leaf.clone()).unwrap();
                let InclusionProofOutput {
                    leaf: proven_leaf,
                    root: proven_root,
                } = inc_proof.evaluate();
                assert_eq!(leaf, proven_leaf);
                assert_eq!(root.clone(), proven_root);

                // Check consistency proofs
                let con_proof = tree.prove_consistency(left_root, root.clone()).unwrap();
                let ConsistencyProofOutput {
                    old_root: proven_old,
                    new_root: proven_new,
                } = con_proof.evaluate().unwrap();
                assert_eq!(proven_old, Some(left_root));
                assert_eq!(proven_new, root.clone());
            }
        }
    }

    #[test]
    fn test_tree_roots_math() {
        // This math is used when computing which roots are available
        type Forrest = InOrderLog<Sha256>;

        assert_eq!(Forrest::broots_for_len(0), vec![]);
        assert_eq!(Forrest::broots_for_len(1), vec![Node(0)]);
        assert_eq!(Forrest::broots_for_len(2), vec![Node(1)]);
        assert_eq!(Forrest::broots_for_len(3), vec![Node(1), Node(4)]);
        assert_eq!(Forrest::broots_for_len(4), vec![Node(3)]);
        assert_eq!(Forrest::broots_for_len(5), vec![Node(3), Node(8)]);
        assert_eq!(Forrest::broots_for_len(6), vec![Node(3), Node(9)]);
        assert_eq!(Forrest::broots_for_len(7), vec![Node(3), Node(9), Node(12)]);
        assert_eq!(Forrest::broots_for_len(8), vec![Node(7)]);
    }
}
