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

use proofs::{InclusionProof, ConsistencyProof};

/// A verifiable log is represented by a merkle tree
/// of its contents and allows you to prove that it is
/// consistent over time and that an entry is present.
pub trait VerifiableLog<D, E>
where
    D: Digest,
    E: AsRef<[u8]>,
{
    /// Construct a new Verifiable Log
    fn new() -> Self;

    /// Get the root representing the state of the log
    fn root(&self) -> Output<D>;

    /// Push a new entry into the log
    fn push(&mut self, entry: E);

    /// Prove that a leaf was present in a given root
    fn prove_inclusion(&self, root: Output<D>, leaf: Output<D>) -> Option<InclusionProof<D>>;

    /// Prove that an old root is consistent with a new one
    fn prove_consistency(
        &self,
        old_root: Output<D>,
        new_root: Output<D>,
    ) -> Option<ConsistencyProof<D>>;
}

pub use in_order::InOrderLog;
