use alloc::vec::Vec;
use anyhow::Error;
use std::collections::HashSet;
use warg_crypto::hash::{Hash, SupportedDigest};

use crate::{
    log::{
        node::Node,
        proof::{ConsistencyProof, InclusionProof},
        sparse_data::SparseLogData,
        LogData,
    },
    protobuf,
};

/// A collection of inclusion proof info
pub struct ProofBundle<D>
where
    D: SupportedDigest,
{
    log_length: u32,
    consistent_lengths: Vec<u32>,
    included_indices: Vec<Node>,
    hashes: Vec<(Node, Hash<D>)>,
}

impl<D> ProofBundle<D>
where
    D: SupportedDigest,
{
    /// Bundles inclusion proofs together
    pub fn bundle(
        consistency_proofs: Vec<ConsistencyProof>,
        inclusion_proofs: Vec<InclusionProof>,
        data: &impl LogData<D>,
    ) -> Result<Self, Error> {
        let mut log_length = None;
        let mut nodes_needed = HashSet::new();

        let mut consistent_lengths = Vec::new();
        for proof in consistency_proofs.iter() {
            consistent_lengths.push(proof.old_length as u32);
            for proof in proof.inclusions()? {
                if let Some(log_length) = log_length {
                    if log_length != proof.log_length {
                        return Err(Error::msg("Bundle must contain proofs for the same root"));
                    }
                } else {
                    log_length = Some(proof.log_length);
                }
                let walk = proof.walk()?;
                for walk_index in walk.nodes {
                    nodes_needed.insert(walk_index);
                }
            }
        }

        let mut included_indices = Vec::new();
        for proof in inclusion_proofs.iter() {
            included_indices.push(proof.leaf);
            if let Some(log_length) = log_length {
                if log_length != proof.log_length {
                    return Err(Error::msg("Bundle must contain proofs for the same root"));
                }
            } else {
                log_length = Some(proof.log_length);
            }
            let walk = proof.walk()?;
            for walk_index in walk.nodes {
                nodes_needed.insert(walk_index);
            }
        }

        let mut nodes_needed: Vec<Node> = nodes_needed.into_iter().collect();
        nodes_needed.sort();
        let mut hashes = Vec::new();
        for node in nodes_needed {
            let hash = data
                .hash_for(node)
                .ok_or_else(|| Error::msg("Necessary hash not found"))?;
            hashes.push((node, hash));
        }

        if let Some(log_length) = log_length {
            Ok(ProofBundle {
                log_length: log_length as u32,
                consistent_lengths,
                included_indices,
                hashes,
            })
        } else {
            return Err(Error::msg("A bundle can not be made from no proofs"));
        }
    }

    /// Splits a bundle into its constituent inclusion proofs
    pub fn unbundle(self) -> (SparseLogData<D>, Vec<ConsistencyProof>, Vec<InclusionProof>) {
        let data = SparseLogData { data: self.hashes };

        let c_proofs = self
            .consistent_lengths
            .into_iter()
            .map(|len| ConsistencyProof {
                old_length: len as usize,
                new_length: self.log_length as usize,
            })
            .collect();

        let i_proofs = self
            .included_indices
            .into_iter()
            .map(|index| InclusionProof {
                log_length: self.log_length as usize,
                leaf: index,
            })
            .collect();

        (data, c_proofs, i_proofs)
    }
}

impl<D> From<ProofBundle<D>> for protobuf::LogProofBundle
where
    D: SupportedDigest,
{
    fn from(value: ProofBundle<D>) -> Self {
        let included_indices = value
            .included_indices
            .into_iter()
            .map(|node| node.0 as u32)
            .collect();
        let hashes = value
            .hashes
            .into_iter()
            .map(|(node, hash)| protobuf::HashEntry {
                index: node.0 as u32,
                hash: hash.as_ref().to_vec(),
            })
            .collect();
        protobuf::LogProofBundle {
            log_length: value.log_length,
            consistent_lengths: value.consistent_lengths,
            included_indices,
            hashes,
        }
    }
}

impl<D> TryFrom<protobuf::LogProofBundle> for ProofBundle<D>
where
    D: SupportedDigest,
{
    type Error = Error;

    fn try_from(value: protobuf::LogProofBundle) -> Result<Self, Self::Error> {
        let included_indices = value
            .included_indices
            .into_iter()
            .map(|index| Node(index as usize))
            .collect();
        let mut hashes = Vec::new();
        for entry in value.hashes {
            hashes.push((Node(entry.index as usize), entry.hash.try_into()?))
        }
        let bundle = ProofBundle {
            log_length: value.log_length,
            consistent_lengths: value.consistent_lengths,
            included_indices,
            hashes,
        };
        Ok(bundle)
    }
}
