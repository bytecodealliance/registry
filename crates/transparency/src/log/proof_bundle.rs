use alloc::vec::Vec;
use anyhow::Error;
use prost::Message;
use std::{collections::HashSet, marker::PhantomData};
use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};
use warg_protobuf::transparency as protobuf;

use crate::log::{
    node::Node,
    proof::{ConsistencyProof, InclusionProof},
    sparse_data::SparseLogData,
    LogData,
};

/// A collection of inclusion proof info
pub struct ProofBundle<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    log_length: u32,
    consistent_lengths: Vec<u32>,
    included_indices: Vec<Node>,
    hashes: Vec<(Node, Hash<D>)>,
    /// Marker for value type
    _digest: PhantomData<D>,
    /// Marker for value type
    _value: PhantomData<V>,
}

impl<D, V> ProofBundle<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// Bundles inclusion proofs together
    pub fn bundle(
        consistency_proofs: Vec<ConsistencyProof<D, V>>,
        inclusion_proofs: Vec<InclusionProof<D, V>>,
        data: &impl LogData<D, V>,
    ) -> Result<Self, Error> {
        let mut log_length = None;
        let mut nodes_needed = HashSet::new();

        let mut consistent_lengths = Vec::new();
        for proof in consistency_proofs.iter() {
            consistent_lengths.push(proof.old_length as u32);
            for proof in proof.inclusions()? {
                if let Some(log_length) = log_length {
                    if log_length != proof.log_length() {
                        return Err(Error::msg("Bundle must contain proofs for the same root"));
                    }
                } else {
                    log_length = Some(proof.log_length());
                }
                let walk = proof.walk()?;
                for walk_index in walk.nodes {
                    nodes_needed.insert(walk_index);
                }
                // Consistency proofs also need the leaf hash for each inclusion
                // proof in order to construct the old root and perform evaluations
                nodes_needed.insert(proof.leaf());
            }
        }

        let mut included_indices = Vec::new();
        for proof in inclusion_proofs.iter() {
            included_indices.push(proof.leaf());
            if let Some(log_length) = log_length {
                if log_length != proof.log_length() {
                    return Err(Error::msg("Bundle must contain proofs for the same root"));
                }
            } else {
                log_length = Some(proof.log_length());
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
                _digest: PhantomData,
                _value: PhantomData,
            })
        } else {
            Err(Error::msg("A bundle can not be made from no proofs"))
        }
    }

    /// Splits a bundle into its constituent inclusion proofs
    #[allow(clippy::type_complexity)]
    pub fn unbundle(
        self,
    ) -> (
        SparseLogData<D, V>,
        Vec<ConsistencyProof<D, V>>,
        Vec<InclusionProof<D, V>>,
    ) {
        let data = SparseLogData::from(self.hashes);

        let c_proofs = self
            .consistent_lengths
            .into_iter()
            .map(|len| ConsistencyProof::new(len as usize, self.log_length as usize))
            .collect();

        let i_proofs = self
            .included_indices
            .into_iter()
            .map(|index| InclusionProof::new(index, self.log_length as usize))
            .collect();

        (data, c_proofs, i_proofs)
    }

    /// Turn a bundle into bytes using protobuf
    pub fn encode(self) -> Vec<u8> {
        let proto: protobuf::LogProofBundle = self.into();
        proto.encode_to_vec()
    }

    /// Parse a bundle from bytes using protobuf
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let proto = protobuf::LogProofBundle::decode(bytes)?;
        let bundle = proto.try_into()?;
        Ok(bundle)
    }
}

impl<D, V> From<ProofBundle<D, V>> for protobuf::LogProofBundle
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn from(value: ProofBundle<D, V>) -> Self {
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
                hash: hash.bytes().to_vec(),
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

impl<D, V> TryFrom<protobuf::LogProofBundle> for ProofBundle<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
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
            _digest: PhantomData,
            _value: PhantomData,
        };
        Ok(bundle)
    }
}
