use std::{
    collections::HashMap,
    vec,
};

use digest::{Digest, Output};

use super::LogForrest;
use super::node::{Node, Side};
use super::proofs::{InclusionProof, ConsistencyProof};

/// A data-structure for building up merkle tree logs based on the DAT paper.
///
/// It represents its data using binary in-order interval numbering.
/// This means that all of the leaf and balanced branch nodes of the tree
/// are stored in one big contiguous array using a particular indexing scheme.
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
#[derive(Debug, Clone)]
pub struct InOrderLogForrest<D, E>
where
    D: Digest,
    E: AsRef<[u8]>,
{
    /// The underlying entries
    entries: Vec<E>,
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
impl<D, E> InOrderLogForrest<D, E>
where
    D: Digest,
    E: AsRef<[u8]>,
{
    /// Computes the height of the tree for a given
    /// log length in number of leaves.
    #[inline]
    fn height_for_len(length: usize) -> u32 {
        if length == 0 {
            0
        } else {
            length.next_power_of_two().trailing_zeros()
        }
    }

    /// Compute the left-most node which has a given height.
    #[inline]
    fn first_node_with_height(height: u32) -> Node {
        Node(2usize.pow(height) - 1)
    }

    /// Compute the balanced roots for a log with a given
    /// log length in number of leaves.
    #[inline]
    fn broots_for_len(length: usize) -> Vec<Node> {
        let height = Self::height_for_len(length);
        let ideal_root = Self::first_node_with_height(height);

        let mut roots: Vec<Node> = Default::default();

        Self::collect_roots(ideal_root, length, &mut roots);

        roots
    }

    #[inline]
    fn collect_roots(node: Node, length: usize, output: &mut Vec<Node>) {
        let height = node.height();

        if node.exists_at_length(length) {
            output.push(node);
            return;
        }

        if height != 0 {
            let (left_child, right_child) = node.children();
            Self::collect_roots(left_child, length, output);
            Self::collect_roots(right_child, length, output);
        }
    }

    fn get_digest(&self, node: Node) -> Output<D> {
        self.tree[node.index()].clone()
    }

    fn set_digest(&mut self, node: Node, digest: Output<D>){
        self.tree[node.index()] = digest;
    }

    fn consistency_proof_node(&self, node: Node, old_length: usize) -> ConsistencyProof<D> {
        if node.exists_at_length(old_length) {
            ConsistencyProof::OldRoot(self.get_digest(node))
        } else if node.nodes_child_exists_at_length(old_length) {
            let (left_index, right_index) = node.children();
            let left = Box::new(self.consistency_proof_node(left_index, old_length));
            let right = Box::new(self.consistency_proof_node(right_index, old_length));
            ConsistencyProof::Hybrid { left, right }
        } else {
            ConsistencyProof::NewHash(self.get_digest(node))
        }
    }
}

impl<D, E> LogForrest<D, E> for InOrderLogForrest<D, E>
where
    D: Digest,
    E: AsRef<[u8]> + Clone
{
    fn new() -> Self {
        InOrderLogForrest {
            entries: vec![],
            tree: vec![],
            root_cache: HashMap::new(),
            leaf_cache: HashMap::new(),
        }
    }

    fn root(&self) -> Output<D> {
        let roots = Self::broots_for_len(self.entries.len());

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

    fn push(&mut self, entry: E) {
        // Compute entry digest
        let mut digest = D::new();
        digest.update(&[0u8]);
        digest.update(&entry);
        let leaf_digest = digest.finalize();

        // Record entry
        self.entries.push(entry);

        // Push spacer (if necessary) and entry digest
        if self.entries.len() != 1 {
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
        self.root_cache.insert(self.root(), self.entries.len());
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

    use crate::log::proofs::{InclusionProofOutput, ConsistencyProofOutput};

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

    fn hexify<T: AsRef<[u8]>>(input: T) -> String {
        input
            .as_ref()
            .into_iter()
            .map(|byte| format!("{:X}", *byte))
            .collect()
    }

    #[test]
    fn test_log_modifications() {
        let data: [&str; 25] = [
            "93", "67", "30", "37", "23", "75", "57", "89", "76", "42", "9", "14", "40", "59", "26",
            "66", "77", "38", "47", "34", "8", "81", "101", "102", "103",
        ];

        let mut tree: InOrderLogForrest<Sha256, &str> = InOrderLogForrest::new();
        let mut roots = Vec::new();

        for i in 0..data.len() {
            tree.push(data[i]);

            let naive_root = naive_merkle::<Sha256, &str>(&data[..i + 1]);

            let tree_root = tree.root();
            assert_eq!(
                tree_root,
                naive_root,
                "at {}: (in-order) {} != (naive) {}",
                i,
                hexify(tree_root),
                hexify(naive_root)
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

            for (j, root) in roots[i..].iter().enumerate() {
                // Check inclusion proofs
                println!("Proving inclusion of {} in {}", i, j);
                let inc_proof = tree.prove_inclusion(root.clone(), leaf.clone()).unwrap();
                let InclusionProofOutput {
                    leaf: proven_leaf,
                    root: proven_root,
                } = inc_proof.evaluate();
                assert_eq!(leaf, proven_leaf);
                assert_eq!(root.clone(), proven_root);

                // Check consistency proofs
                println!("Proving consistency between {} and {}", i, j);
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
    fn test_tree_height_math() {
        type Forrest = InOrderLogForrest<Sha256, &'static str>;

        // Heights for numbers of entries
        assert_eq!(Forrest::height_for_len(1), 0);
        assert_eq!(Forrest::height_for_len(2), 1);
        assert_eq!(Forrest::height_for_len(3), 2);
        assert_eq!(Forrest::height_for_len(4), 2);
        assert_eq!(Forrest::height_for_len(5), 3);
        assert_eq!(Forrest::height_for_len(6), 3);
        assert_eq!(Forrest::height_for_len(7), 3);
        assert_eq!(Forrest::height_for_len(8), 3);
    }

    #[test]
    fn test_tree_roots_math() {
        // This math is used when computing which roots are available
        type Forrest = InOrderLogForrest<Sha256, &'static str>;

        // First node with each height
        assert_eq!(Forrest::first_node_with_height(0), Node(0));
        assert_eq!(Forrest::first_node_with_height(1), Node(1));
        assert_eq!(Forrest::first_node_with_height(2), Node(3));
        assert_eq!(Forrest::first_node_with_height(3), Node(7));
        assert_eq!(Forrest::first_node_with_height(4), Node(15));

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
