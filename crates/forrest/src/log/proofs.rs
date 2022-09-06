use alloc::vec::Vec;
use alloc::boxed::Box;

use digest::{Digest, Output};

use super::node::Side;

/// Error describing proof that failed validation/evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum ProofError {
    NotTrue,
    InvalidStructure,
}

/// A proof that a leaf is present for a root
#[derive(Debug, Clone, PartialEq)]
pub struct InclusionProof<D>
where
    D: Digest,
{
    pub(crate) leaf: Output<D>,
    pub(crate) path: Vec<(Side, Output<D>)>,
}

pub struct InclusionProofOutput<D>
where
    D: Digest,
{
    pub leaf: Output<D>,
    pub root: Output<D>,
}

impl<D> InclusionProof<D>
where
    D: Digest,
{
    pub fn evaluate(&self) -> InclusionProofOutput<D> {
        let root = self
            .path
            .iter()
            .fold(self.leaf.clone(), |old, (side, new)| {
                let (lhs, rhs) = match side {
                    Side::Left => (new, &old),
                    Side::Right => (&old, new),
                };
                let mut digest = D::new();
                digest.update(&[1u8]);
                digest.update(lhs);
                digest.update(rhs);
                digest.finalize()
            });
        InclusionProofOutput {
            leaf: self.leaf.clone(),
            root,
        }
    }
}

// The proof needs to demonstrate that each balanced root of the old tree
/// is included in the overall root of the new tree.
///
/// This is done by constructing a merkle tree where
/// * the root is the new tree root, and
/// * the leaves are either balanced roots of the old tree or hashes in the new.
///
/// A node in the consistency proof tree.
///
/// To be structurally valid tree all branches must have these properties.
/// * The left child is either an OldRoot or another Hybrid node.
/// * The right child is a NewHash node.
///
/// If a malformed `ConsistencyProofNode` tree is validated,
/// the result will be a `ProofError::InvalidStructure`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsistencyProof<D>
where
    D: Digest,
{
    OldRoot(Output<D>),
    NewHash(Output<D>),
    Hybrid {
        left: Box<ConsistencyProof<D>>,
        right: Box<ConsistencyProof<D>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConsistencyProofOutput<D>
where
    D: Digest,
{
    pub old_root: Option<Output<D>>,
    pub new_root: Output<D>,
}

impl<D> ConsistencyProof<D>
where
    D: Digest,
{
    pub fn evaluate(&self) -> Result<ConsistencyProofOutput<D>, ProofError> {
        match self {
            ConsistencyProof::OldRoot(digest) => Ok(ConsistencyProofOutput {
                old_root: Some(digest.clone()),
                new_root: digest.clone(),
            }),
            ConsistencyProof::NewHash(digest) => Ok(ConsistencyProofOutput {
                old_root: None,
                new_root: digest.clone(),
            }),
            ConsistencyProof::Hybrid { left, right } => {
                let ConsistencyProofOutput {
                    old_root: left_old_root,
                    new_root: left_new_root,
                } = left.evaluate()?;
                let ConsistencyProofOutput {
                    old_root: right_old_root,
                    new_root: right_new_root,
                } = right.evaluate()?;

                let old_root = match &(left_old_root, right_old_root) {
                    (Some(left_old), Some(right_old)) => {
                        let mut old_digest = D::new();
                        old_digest.update(&[1u8]);
                        old_digest.update(left_old);
                        old_digest.update(right_old);
                        Some(old_digest.finalize())
                    }
                    (Some(left_old), None) => Some(left_old.clone()),
                    (None, None) => None,
                    _ => return Err(ProofError::InvalidStructure),
                };

                let new_root = {
                    let mut new_digest = D::new();
                    new_digest.update(&[1u8]);
                    new_digest.update(&left_new_root);
                    new_digest.update(&right_new_root);
                    new_digest.finalize()
                };

                Ok(ConsistencyProofOutput {
                    old_root: old_root.clone(),
                    new_root,
                })
            }
        }
    }
}