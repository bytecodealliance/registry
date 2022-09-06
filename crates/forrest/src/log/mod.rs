use digest::{Digest, Output};

mod in_order;
mod node;
mod proofs;

use proofs::{InclusionProof, ConsistencyProof};

pub trait LogForrest<D, E>
where
    D: Digest,
    E: AsRef<[u8]>,
{
    fn new() -> Self;

    fn root(&self) -> Output<D>;

    fn push(&mut self, entry: E);

    fn prove_inclusion(&self, root: Output<D>, leaf: Output<D>) -> Option<InclusionProof<D>>;

    fn prove_consistency(
        &self,
        old_root: Output<D>,
        new_root: Output<D>,
    ) -> Option<ConsistencyProof<D>>;
}

pub use in_order::InOrderLogForrest;