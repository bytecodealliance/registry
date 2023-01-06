// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use alloc::vec::Vec;
use core::{iter::repeat, marker::PhantomData, ops::Deref};

use digest::Digest;
use serde::{Deserialize, Serialize};

use super::path::Path;
use crate::hash::Hash;

/// An inclusion proof of the specified value in a map
///
/// # Compression
///
/// Since the depth of a tree is always `n` and a proof needs to contain all
/// branch node peers from the leaf to the root, a proof should contain `n`
/// hashes. However, several strategies can be used to compresss a proof;
/// saving both memory and bytes on the wire.
///
/// First, the hash of the item and the root are known by both sides and can
/// be omitted.
///
/// Second, sparse peers can be represented by `None`. Since we take references
/// to the hashes, Rust null-optimization is used.
///
/// Third, since sparse peers are more likely at the bottom of the tree, we
/// can omit all leading sparse peers. The verifier can dynamically reconstruct
/// them during verification.
#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "H: Serialize, V: serde_bytes::Serialize"))]
#[serde(bound(deserialize = "H: Deserialize<'de>, V: serde_bytes::Deserialize<'de>"))]
pub struct Proof<D: Digest, H, V> {
    #[serde(skip)]
    pub(crate) digest: PhantomData<D>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) peers: Vec<Option<H>>,

    #[serde(with = "serde_bytes")]
    pub(crate) value: V,
}

impl<D: Digest, H: Deref<Target = Hash<D>>, V: AsRef<[u8]>> Proof<D, H, V> {
    /// Verifies a proof for a given map and key.
    #[must_use]
    pub fn verify<Q: ?Sized + AsRef<[u8]>>(&self, root: &Hash<D>, key: &Q) -> bool {
        // Get the path from bottom to top.
        let path = Path::<D>::from(key);

        // Determine how many empty leading peers there will be.
        let fill = repeat(None).take(path.len() - self.peers.len());

        // Calculate the leaf hash.
        let mut hash = path.hash(&self.value);

        // Loop over each side and peer.
        let peers = fill.chain(self.peers.iter().map(|x| x.as_deref()));
        for (side, peer) in path.rev().zip(peers) {
            // Reconstruct the fork.
            let fork = match side {
                false => [Some(&hash), peer],
                true => [peer, Some(&hash)],
            };

            // Calculate the hash inputs.
            let (x, l, r): (u8, &[u8], &[u8]) = match fork {
                [Some(l), Some(r)] => (0b11, l, r),
                [Some(l), None] => (0b10, l, &[]),
                [None, Some(r)] => (0b01, &[], r),
                [None, None] => (0b00, &[], &[]),
            };

            // Calculate the hash at this level.
            hash = D::new()
                .chain_update(&[x])
                .chain_update(l)
                .chain_update(r)
                .finalize()
                .into();
        }

        &hash == root
    }
}

#[test]
#[cfg(test)]
fn test() {
    use serde_bytes::ByteBuf;
    use sha2::Sha256;

    let a = super::Map::<Sha256, &str, &[u8]>::default();
    let b = a.insert("foo", b"bar");
    let c = b.insert("baz", b"bat");
    let p = c.prove("baz").unwrap();

    let mut buf = Vec::new();
    ciborium::ser::into_writer(&p, &mut buf).unwrap();

    for byte in &buf {
        std::eprint!("{:02x}", byte);
    }
    std::eprintln!();

    let proof: Proof<Sha256, Hash<Sha256>, ByteBuf> = ciborium::de::from_reader(&buf[..]).unwrap();
    let peers = proof.peers.iter().map(|x| x.as_ref()).collect::<Vec<_>>();
    assert_eq!(p.peers, peers);
    assert_eq!(*p.value, *proof.value);
}
