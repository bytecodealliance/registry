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
//!
use digest::Digest;

/// Logic for manipulating log tree node indices
pub mod node;
/// Logic for constructing and validating proofs
pub mod proofs;
mod vec_log;

use self::node::Node;
use crate::hash::Hash;

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
pub trait VerifiableLog<D>
where
    D: Digest,
{
    /// Get the root representing the state of the log
    fn root(&self) -> Hash<D>;

    /// Get the checkpoint for the current log state
    fn checkpoint(&self) -> Checkpoint;

    /// Get the root hash for a given checkpoint.
    /// None if the log has not yet reached the checkpoint.
    fn root_at(&self, point: Checkpoint) -> Option<Hash<D>>;

    /// Get the hash for a given node
    /// None if node does not yet exist
    fn hash_for(&self, node: Node) -> Option<Hash<D>>;

    /// Push a new entry into the log
    fn push(&mut self, entry: impl AsRef<[u8]>) -> (Checkpoint, Node);
}

/// A point in the history of a log, represented by its length
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Checkpoint(pub usize);

impl Checkpoint {
    /// The length of the log at this checkpoint
    pub fn length(&self) -> usize {
        self.0
    }
}

/// A collection of hash data
pub trait HashProvider<D>
where
    D: Digest,
{
    /// Does this hash exist in the collection?
    fn has_hash(&self, node: Node) -> bool;

    /// What is the hash for this node?
    fn hash_for(&self, node: Node) -> Option<Hash<D>>;
}

pub use vec_log::VecLog;

/// Compute the hash for an empty tree using a given Digest algorithm.
pub fn hash_empty<D: Digest>() -> Hash<D> {
    D::new().finalize().into()
}

/// Compute the hash for a leaf in a tree using a given Digest algorithm.
pub fn hash_leaf<D: Digest>(data: impl AsRef<[u8]>) -> Hash<D> {
    let mut digest = D::new();
    digest.update(&[0u8]);
    digest.update(data);
    digest.finalize().into()
}

/// Compute the hash for a branch in a tree using a given Digest algorithm.
pub fn hash_branch<D: Digest>(left: impl AsRef<[u8]>, right: impl AsRef<[u8]>) -> Hash<D> {
    let mut digest = D::new();
    digest.update(&[1u8]);
    digest.update(left);
    digest.update(right);
    digest.finalize().into()
}
