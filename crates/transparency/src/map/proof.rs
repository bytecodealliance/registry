use alloc::vec::Vec;
use core::marker::PhantomData;
use std::iter::repeat;

use warg_crypto::{
    hash::{AnyHash, Hash, SupportedDigest},
    VisitBytes,
};

use super::{
    map::hash_branch,
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
pub struct Proof<D, V>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    value: PhantomData<V>,
    pub peers: Vec<Option<Hash<D>>>,
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
        self.peers.push(peer);
    }

    /// Computes the root obtained by evaluating this inclusion proof with the given leaf
    pub fn evaluate(&self, key: AnyHash, value: &V) -> Hash<D> {
        // Get the path from bottom to top.
        let hash_key: Hash<D> = key.try_into().unwrap();
        let path = ReversePath::<D>::new(hash_key);

        let fill = repeat(None).take(256 - self.peers.len());
        // Calculate the leaf hash.
        let mut hash = Hash::of(value);

        // // Loop over each side and peer.
        let peers = fill.chain(self.peers.iter().cloned());
        for (i, (side, peer)) in path.zip(peers).enumerate() {
            match &peer {
                Some(_) => {
                    hash = match side {
                        Side::Left => hash_branch(hash, peer.unwrap()),
                        Side::Right => hash_branch(peer.unwrap(), hash),
                    };
                }
                None => match side {
                    Side::Left => hash = hash_branch(hash, D::empty_tree_hash(i)),
                    Side::Right => {
                        hash = hash_branch(D::empty_tree_hash(i), hash);
                    }
                },
            }
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
    use warg_crypto::hash::{AnyHash, Hash};

    #[test]
    fn test_proof_evaluate() {
        use warg_crypto::hash::Sha256;

        let a = crate::map::Map::<Sha256, &str, &[u8]>::default();
        // let b = a.insert("foo", b"bar");
        // let c = b.insert("baz", b"bat");
        let b = a.insert(AnyHash::from(Hash::<Sha256>::of("foo")), b"bar");
        let c = b.insert(AnyHash::from(Hash::<Sha256>::of("baz")), b"bat");

        let root = c.root().clone();

        // let p = c.prove(&"baz").unwrap();
        let p = c.prove(AnyHash::from(Hash::<Sha256>::of(&"baz"))).unwrap();

        // assert_eq!(root, p.evaluate(&"baz", &b"bat".as_slice()));
        // assert_ne!(root, p.evaluate(&"other", &b"bar".as_slice()));
        assert_eq!(
            root,
            p.evaluate(AnyHash::from(Hash::<Sha256>::of("baz")), &b"bat".as_slice())
        );
        assert_ne!(
            root,
            p.evaluate(
                AnyHash::from(Hash::<Sha256>::of("other")),
                &b"bar".as_slice()
            )
        );
    }
}
