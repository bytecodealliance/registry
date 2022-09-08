//! Immutable Log w/ Inclusion and Consistency Proofs
//!
//! The main trait in this module is [`VerifiableLog`].
//! A verifiable log is a [Merkle Tree][0] of its entries,
//! which allows it to support inclusion and consistency proofs.
//! Implementations of this API must compute the tree in
//! the manner described by [RFC 6962 - Certificate Transparency][1].
//!
//! The only implementation in this module is [`InOrderLog`],
//! which is a [`VerifiableLog`] whose contents are structured
//! using binary in-order interval numbering as described in
//! [Dat - Distributed Dataset Synchronization and Versioning][2].
//!
//! [0]: https://en.wikipedia.org/wiki/Merkle_tree
//! [1]: https://www.rfc-editor.org/rfc/rfc6962
//! [2]: https://www.researchgate.net/publication/326120012_Dat_-_Distributed_Dataset_Synchronization_And_Versioning
use digest::{Digest, Output};
mod in_order;
mod node;
mod proofs;

use proofs::{ConsistencyProof, InclusionProof};

/// A verifiable log is represented by a merkle tree
/// of its contents and allows you to prove that it is
/// consistent over time and that an entry is present.
pub trait VerifiableLog<D>
where
    D: Digest,
{
    /// Get the root representing the state of the log
    fn root(&self) -> Output<D>;

    /// Push a new entry into the log
    fn push(&mut self, entry: impl AsRef<[u8]>);

    /// Prove that a leaf was present in a given root
    fn prove_inclusion(&self, root: Output<D>, leaf: Output<D>) -> Result<InclusionProof<D>, InclusionProofError>;

    /// Prove that an old root is consistent with a new one
    fn prove_consistency(
        &self,
        old_root: Output<D>,
        new_root: Output<D>,
    ) -> Result<ConsistencyProof<D>, ConsistencyProofError>;
}

/// Th errors that may occur when computing an inclusion proof
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InclusionProofError {
    /// The leaf provided is more recent than the specified root
    LeafNotInRoot,
    /// The root was not found
    RootNotKnown,
    /// The leaf was not found
    LeafNotKnown,
}

/// The errors that may occur when computing a consistency proof
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConsistencyProofError {
    /// The new and old roots are swapped
    IncorrectOrdering,
    /// The old root was not found
    OldRootNotKnown,
    /// The new root was not found
    NewRootNotKnown,
}

pub use in_order::InOrderLog;
