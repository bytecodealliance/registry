use alloc::vec::Vec;
use anyhow::Error;
use prost::Message;
use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};
use warg_protobuf::transparency as protobuf;

use crate::map::proof::Proof;

/// A collection of inclusion proof info
pub struct ProofBundle<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    proofs: Vec<Proof<D, K, V>>,
}

impl<D, K, V> ProofBundle<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    /// Bundles inclusion proofs together
    pub fn bundle(proofs: Vec<Proof<D, K, V>>) -> Self {
        ProofBundle { proofs }
    }

    /// Splits a bundle into its constituent inclusion proofs
    pub fn unbundle(self) -> Vec<Proof<D, K, V>> {
        self.proofs
    }

    /// Turn a bundle into bytes using protobuf
    pub fn encode(self) -> Vec<u8> {
        let proto: protobuf::MapProofBundle = self.into();
        proto.encode_to_vec()
    }

    /// Parse a bundle from bytes using protobuf
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        let proto = protobuf::MapProofBundle::decode(bytes)?;
        let bundle = proto.try_into()?;
        Ok(bundle)
    }
}

impl<D, K, V> From<ProofBundle<D, K, V>> for protobuf::MapProofBundle
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn from(value: ProofBundle<D, K, V>) -> Self {
        let proofs = value.proofs.into_iter().map(|proof| proof.into()).collect();
        protobuf::MapProofBundle { proofs }
    }
}

impl<D, K, V> From<Proof<D, K, V>> for protobuf::MapInclusionProof
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn from(value: Proof<D, K, V>) -> Self {
        let peers: Vec<Option<Hash<D>>> = value.into();
        protobuf::MapInclusionProof {
            hashes: peers.into_iter().map(|h| h.into()).collect(),
        }
    }
}

impl<D, K, V> TryFrom<protobuf::MapProofBundle> for ProofBundle<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    type Error = Error;

    fn try_from(value: protobuf::MapProofBundle) -> Result<Self, Self::Error> {
        let mut proofs = Vec::new();
        for entry in value.proofs {
            proofs.push(entry.try_into()?);
        }
        let bundle = ProofBundle { proofs };
        Ok(bundle)
    }
}

impl<D, K, V> TryFrom<protobuf::MapInclusionProof> for Proof<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    type Error = Error;

    fn try_from(value: protobuf::MapInclusionProof) -> Result<Self, Self::Error> {
        let peers: Result<Vec<Option<Hash<D>>>, Error> =
            value.hashes.into_iter().map(|h| h.try_into()).collect();
        let proof = Proof::new(peers?);
        Ok(proof)
    }
}
