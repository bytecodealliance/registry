use alloc::vec::Vec;
use anyhow::Error;
use prost::Message;
use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use crate::{map::proof::Proof, protobuf};

/// A collection of inclusion proof info
pub struct ProofBundle<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    proofs: Vec<Proof<D, V>>,
}

impl<D, V> ProofBundle<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    /// Bundles inclusion proofs together
    pub fn bundle(proofs: Vec<Proof<D, V>>) -> Self {
        ProofBundle { proofs }
    }

    /// Splits a bundle into its constituent inclusion proofs
    pub fn unbundle(self) -> Vec<Proof<D, V>> {
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

impl<D, V> From<ProofBundle<D, V>> for protobuf::MapProofBundle
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn from(value: ProofBundle<D, V>) -> Self {
        let proofs = value.proofs.into_iter().map(|proof| proof.into()).collect();
        protobuf::MapProofBundle { proofs }
    }
}

impl<D, V> From<Proof<D, V>> for protobuf::MapInclusionProof
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn from(value: Proof<D, V>) -> Self {
        let peers: Vec<Option<Hash<D>>> = value.into();
        protobuf::MapInclusionProof {
            hashes: peers.into_iter().map(|h| h.into()).collect(),
        }
    }
}

impl<D> From<Option<Hash<D>>> for protobuf::OptionalHash
where
    D: SupportedDigest,
{
    fn from(value: Option<Hash<D>>) -> Self {
        protobuf::OptionalHash {
            hash: value.map(|h| h.bytes().to_vec()),
        }
    }
}

impl<D, V> TryFrom<protobuf::MapProofBundle> for ProofBundle<D, V>
where
    D: SupportedDigest,
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

impl<D, V> TryFrom<protobuf::MapInclusionProof> for Proof<D, V>
where
    D: SupportedDigest,
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

impl<D> TryFrom<protobuf::OptionalHash> for Option<Hash<D>>
where
    D: SupportedDigest,
{
    type Error = Error;

    fn try_from(value: protobuf::OptionalHash) -> Result<Self, Self::Error> {
        let hash = match value.hash {
            Some(h) => Some(h.try_into()?),
            None => None,
        };
        Ok(hash)
    }
}
