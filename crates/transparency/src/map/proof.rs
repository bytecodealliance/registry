use alloc::vec::Vec;
use core::marker::PhantomData;
use std::iter::repeat;

use warg_crypto::{
    hash::{Hash, SupportedDigest},
    VisitBytes,
};

use super::{
    map::{hash_branch, hash_leaf},
    path::{ReversePath, Side},
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
pub struct Proof<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    key: PhantomData<K>,
    value: PhantomData<V>,
    /// Sibling node hashes needed to construct a proof
    pub peers: Vec<Option<Hash<D>>>,
}

impl<D, K, V> Proof<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    pub(crate) fn new(peers: Vec<Option<Hash<D>>>) -> Self {
        Self {
            key: PhantomData,
            value: PhantomData,
            peers,
        }
    }

    pub(crate) fn push(&mut self, peer: Option<Hash<D>>) {
        self.peers.push(peer);
    }

    /// Computes the root obtained by evaluating this inclusion proof with the given leaf
    pub fn evaluate(&self, key: &K, value: &V) -> Hash<D> {
        // Get the path from bottom to top.
        let path = ReversePath::<D>::new(Hash::of(key));

        let fill = repeat(None).take(256 - self.peers.len());
        // Calculate the leaf hash.
        let mut hash = hash_leaf(value);

        // Loop over each side and peer.
        let peers = fill.chain(self.peers.iter().cloned());
        for (i, (side, peer)) in path.zip(peers).enumerate() {
            match &peer {
                Some(_) => {
                    hash = match side {
                        Side::Left => hash_branch(&hash, &peer.unwrap()),
                        Side::Right => hash_branch(&peer.unwrap(), &hash),
                    };
                }
                None => match side {
                    Side::Left => hash = hash_branch(&hash, D::empty_tree_hash(i)),
                    Side::Right => {
                        hash = hash_branch(D::empty_tree_hash(i), &hash);
                    }
                },
            }
        }

        hash
    }
}

impl<D, K, V> From<Proof<D, K, V>> for Vec<Option<Hash<D>>>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn from(value: Proof<D, K, V>) -> Self {
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

        let p = c.prove("baz").unwrap();

        assert_eq!(root, p.evaluate(&"baz", &b"bat".as_slice()));
        assert_ne!(root, p.evaluate(&"other", &b"bar".as_slice()));
    }
}
