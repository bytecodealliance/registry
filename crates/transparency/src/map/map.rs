use core::fmt::{Debug, Formatter};
use std::borrow::Borrow;
use std::marker::PhantomData;

use warg_crypto::hash::{AnyHash, Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::link::Link;
use super::node::Node;
use super::path::{Path, ReversePath};
use super::proof::Proof;

/// Immutable Map w/ Inclusion Proofs
///
/// # Usage
///
/// Using a [`Map`] should feel similar to using a `HashMap`. All maps begin
/// by creating an empty map and then populating it with items. Each time you
/// insert to a map, it creates a new map derived from the prior map. Once
/// items are inserted, you can get an inclusion proof that demonstrates the
/// presence of a key/value in a tree.
///
///
/// # Design
///
/// A [`Map`] is a key/value store backed by a Merkle tree. Its values are
/// stored in a binary tree. A cryptographic hash function is applied to the
/// key and its resulting bits are interpreted as a path to descend the tree.
/// This implies that the depth of the tree is `n` where `n` is the number of
/// bits in the cryptographic hash function. Because this would cause the tree
/// to have `2^(n+1) - 1` entries (far too many!), the tree is sparse and
/// only creates nodes as necessary to represent the contents in the tree.
///
pub struct Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    pub link: Link<D>,
    len: usize,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    pub(crate) fn new(link: Link<D>, len: usize) -> Self {
        Self {
            link,
            len,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Clone for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
            len: self.len,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Default for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    fn default() -> Self {
        Self {
            link: Link::new(Node::Empty(256)),
            len: 0,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Eq for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
}

impl<D, K, V> PartialEq for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    fn eq(&self, other: &Self) -> bool {
        self.link.hash() == other.link.hash()
    }
}

impl<D, K, V> core::hash::Hash for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.link.hash().hash(state);
    }
}

impl<D, K, V> Debug for Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("Map({:?})", self.link.hash()))
    }
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest + Debug,
    K: VisitBytes + Debug,
    V: VisitBytes,
{
    /// The hash of the root of the tree.
    ///
    /// This uniquely identifies the map and its contents.
    pub fn root(&self) -> &Hash<D> {
        self.link.hash()
    }

    /// The number of items in the map.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether or not the map is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Gets the value for a given key and a proof of its presence in this map.
    pub fn prove(&self, key: AnyHash) -> Option<Proof<D, V>>
where
        // K: Borrow<Q>,
        // Q: VisitBytes,
    {
        let hash: Hash<D> = key.try_into().unwrap();
        self.link.node().prove(Path::new(hash), None)
    }

    /// Insert a value into the map, creating a new map.
    ///
    /// This replaces any existing items with the same key.
    pub fn insert(&self, key: AnyHash, val: V) -> Self {
        // pub fn insert(&self, key: K, val: V) -> Self {
        // dbg!("MAP INSERTION", &key);
        // let new_key = key.visit(visitor);

        // let hash = Hash::<D>::from(&key);
        // let key_hash = match key {
        //   Hash<D> => key,
        //   _ => Hash::<D>::of(key)
        // };
        let new_key: Hash<D> = key.try_into().unwrap();
        let mut path = Path::new(new_key.clone());
        // let mut path = Path::new(&key);
        let mut reversed = ReversePath::new(new_key.clone());
        // let mut reversed = ReversePath::new(&key);
        let (node, new) = self
            .link
            .node()
            // .insert(&mut path, &mut reversed, Hash::of(key), Hash::of(val));
            .insert(&mut path, &mut reversed, new_key, Hash::of(val));
        // match &node {
        //   Node::Fork(fork) => {
        //     dbg!(fork);
        //   }
        //   _ => {}
        // }
        Self::new(Link::new(node), self.len + usize::from(new))
    }

    /// Inserts all key/value pairs into the map, creating a new map.
    pub fn extend(&self, iter: impl IntoIterator<Item = (AnyHash, V)>) -> Self {
        let mut here = self.clone();

        for (key, val) in iter {
            let hash: Hash<D> = key.try_into().unwrap();
            let mut path = Path::new(hash.clone());
            let mut reversed: ReversePath<D> = ReversePath::new(hash.clone());
            let (node, new) =
                here.link
                    .node()
                    .insert(&mut path, &mut reversed, Hash::of(hash), Hash::of(val));
            here = Self::new(Link::new(node), here.len + usize::from(new));
        }

        here
    }
}

pub(crate) fn hash_branch<D>(lhs: Option<Hash<D>>, rhs: Option<Hash<D>>) -> Hash<D>
where
    D: SupportedDigest,
{
    // dbg!(lhs.clone().unwrap(), rhs.clone().unwrap());
    Hash::of((0b1, lhs.unwrap(), rhs.unwrap()))
}

// [crates/transparency/src/map/map.rs:223] lhs.clone().unwrap() = Hash<Sha256>(c21a4fdf2cb3f9fdc6160ef05b7ddcbd1b656ac21f4f2fdb83f6eb231fb1dfef)
// [crates/transparency/src/map/map.rs:223] rhs.clone().unwrap() = Hash<Sha256>(16fc8d04ab903d99afeccb72cbdc5020fbae2ac2642716308bf0340584b89bbb)