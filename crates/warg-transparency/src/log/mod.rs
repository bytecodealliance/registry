//! Immutable Log w/ Inclusion and Consistency Proofs
//!
//! The main trait in this module is [`VerifiableLog`],
//! which defines the API of a log where inclusion
//! and consistency can be verified.
//!
//! Implementations:
//! * [`InOrderLog`] -
//! The only implementation in this module is ,
//! which is a [`VerifiableLog`] whose contents are structured
//! using binary in-order interval numbering as described in
//! [Dat - Distributed Dataset Synchronization and Versioning][2].

mod node;
/// Logic for constructing and validating proofs
mod proof;
mod proof_bundle;
mod sparse_data;
mod stack_log;
mod vec_log;

use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

pub use node::{Node, Side};
pub use proof::{
    ConsistencyProof, ConsistencyProofError, InclusionProof, InclusionProofError,
    InclusionProofWalk,
};
pub use proof_bundle::ProofBundle;
pub use proof_bundle::ProofBundle as LogProofBundle;
pub use stack_log::StackLog;
pub use vec_log::VecLog;

/// A [merkle tree][0] log data type based on [DAT][1].
/// where the merkle tree computation is conformant to
/// [RFC 6962 - Certificate Transparency][2]. This allows
/// you to efficiently append data and then verify that
/// it the log is consistent over time and contains a
/// given entry.
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
pub trait LogBuilder<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// Get the checkpoint (hash and length) of the log at this point.
    fn checkpoint(&self) -> Checkpoint<D>;

    /// Push a new entry into the log.
    fn push(&mut self, entry: &V) -> Node;
}

/// A point in the history of a log, represented by its length
#[derive(Debug, Clone, PartialOrd, Ord)]
pub struct Checkpoint<D>
where
    D: SupportedDigest,
{
    root: Hash<D>,
    length: usize,
}

impl<D> Checkpoint<D>
where
    D: SupportedDigest,
{
    /// The root hash of the log at this checkpoint
    pub fn root(&self) -> Hash<D> {
        self.root.clone()
    }

    /// The length of the log at this checkpoint
    pub fn length(&self) -> usize {
        self.length
    }
}

impl<D> Eq for Checkpoint<D> where D: SupportedDigest {}

impl<D> PartialEq for Checkpoint<D>
where
    D: SupportedDigest,
{
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.length == other.length
    }
}

/// A collection of hash data
pub trait LogData<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// Does this hash exist in the collection?
    fn has_hash(&self, node: Node) -> bool;

    /// Get the hash for a given node
    /// None if node does not yet exist
    fn hash_for(&self, node: Node) -> Option<Hash<D>>;

    /// Construct an inclusion proof for this log
    fn prove_inclusion(&self, leaf: Node, log_length: usize) -> InclusionProof<D, V> {
        InclusionProof::new(leaf, log_length)
    }

    /// Construct a consistency proof for this log
    fn prove_consistency(&self, old_length: usize, new_length: usize) -> ConsistencyProof<D, V> {
        ConsistencyProof::new(old_length, new_length)
    }
}

/// Compute the hash for an empty tree using a given Digest algorithm.
pub(crate) fn hash_empty<D: SupportedDigest>() -> Hash<D> {
    Hash::of(())
}

/// Compute the hash for a leaf in a tree using a given Digest algorithm.
pub(crate) fn hash_leaf<D: SupportedDigest>(data: impl VisitBytes) -> Hash<D> {
    let input = (0u8, data);
    Hash::of(&input)
}

/// Compute the hash for a branch in a tree using a given Digest algorithm.
pub(crate) fn hash_branch<D: SupportedDigest>(
    left: impl VisitBytes,
    right: impl VisitBytes,
) -> Hash<D> {
    let input = (1u8, left, right);
    Hash::of(&input)
}
