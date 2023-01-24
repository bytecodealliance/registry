use alloc::vec::Vec;
use core::{iter::repeat, marker::PhantomData};

use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::{
    map::{hash_branch, hash_leaf},
    path::{Path, Side},
};

/// An inclusion proof of the specified value in a map
///
/// # Compression
///
/// Since the depth of a tree is always `n` and a proof needs to contain all
/// branch node peers from the leaf to the root, a proof should contain `n`
/// hashes. However, several strategies can be used to compress a proof;
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
pub struct Proof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    value: PhantomData<V>,
    peers: Vec<Option<Hash<D>>>,
}

impl<D, V> Proof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    pub(crate) fn new(peers: Vec<Option<Hash<D>>>) -> Self {
        Self {
            value: PhantomData,
            peers,
        }
    }

    pub(crate) fn push(&mut self, peer: Option<Hash<D>>) {
        // This is an optimization. The size of a proof is always
        // known: it is the number of bits in the digest. Therefore,
        // we can skip all leading nodes with no peer. The validator,
        // can reconstruct this.
        if !self.peers.is_empty() || peer.is_some() {
            self.peers.push(peer);
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.peers.len()
    }

    /// Computes the root obtained by evaluating this inclusion proof with the given leaf
    pub fn evaluate<K: ?Sized + VisitBytes>(&self, key: &K, value: &V) -> Hash<D> {
        // Get the path from bottom to top.
        let path = Path::<D>::new(key);

        // Determine how many empty leading peers there will be.
        let fill = repeat(None).take(path.len() - self.peers.len());

        // Calculate the leaf hash.
        let mut hash = hash_leaf(key, value);

        // Loop over each side and peer.
        let peers = fill.chain(self.peers.iter().cloned());
        for (side, peer) in path.rev().zip(peers) {
            hash = match side {
                Side::Left => hash_branch(Some(hash), peer),
                Side::Right => hash_branch(peer, Some(hash)),
            };
        }

        hash
    }
}

impl<D, V> From<Proof<D, V>> for Vec<Option<Hash<D>>>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    fn from(value: Proof<D, V>) -> Self {
        value.peers
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_proof_evaluate() {
        use warg_crypto::hash::Sha256;

        let a = crate::map::Map::<Sha256, &str, &[u8]>::default();
        let b = a.insert("foo", b"bar");
        let c = b.insert("baz", b"bat");

        let root = c.root().clone();

        let p = c.prove(&"baz").unwrap();

        assert_eq!(root.clone(), p.evaluate(&"baz", &b"bat".as_slice()));
        assert_ne!(root.clone(), p.evaluate(&"other", &b"bar".as_slice()));
    }
}
