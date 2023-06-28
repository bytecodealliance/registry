use core::fmt::{Debug, Formatter};
use std::marker::PhantomData;

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::link::Link;
use super::node::Node;
use super::path::Path;
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
/// ```rust
/// use warg_transparency::map::Map;
/// use sha2::Sha256;
///
/// let a = Map::<Sha256, &str, &str>::default();
/// let b = a.insert("foo", "bar");
/// let c = b.extend([("baz", "bat"), ("foo", "qux")]);
///
/// let proof = c.prove(&"foo").unwrap();
/// assert_eq!(c.root().clone(), proof.evaluate(&"foo", &"qux"));
/// ```
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
/// ## Hashing Strategy
///
/// ### Leaf Nodes
///
/// Leaf node hashes are calculated using the one-byte `0` prefix to prevent collisions with branch hashes.
///
/// ```hash
/// hash(Leaf(value)) = hash(0x00 || <value>)
/// ```
///
/// For leaf nodes that semantically "contain no value", the `<value>` provided is empty i.e. the zero-length byte sequence.
///
///
/// ### Branch Nodes
///
/// Branch node hashes are calculated using the one-byte `1` prefix to prevent collisions with branch hashes.
/// ```hash
/// hash(Branch(left, right)) = hash(0x01 || hash(<left>) || hash(<right>))
/// ```
pub struct Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
{
    link: Link<D>,
    len: usize,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
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
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
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
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
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
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
{
}

impl<D, K, V> PartialEq for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
{
    fn eq(&self, other: &Self) -> bool {
        self.link.hash() == other.link.hash()
    }
}

impl<D, K, V> core::hash::Hash for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.link.hash().hash(state);
    }
}

impl<D, K, V> Debug for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone,
    V: VisitBytes + Clone,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("Map({:?})", self.link.hash()))
    }
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes + Clone + ?Sized,
    V: VisitBytes + Clone,
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
    pub fn prove(&self, key: K) -> Option<Proof<D, K, V>>
where {
        self.link.node().prove(Path::new(&Hash::of(key)))
    }

    /// Insert a value into the map, creating a new map.
    ///
    /// This replaces any existing items with the same key.
    pub fn insert(&self, key: K, val: V) -> Self {
        let key_hash = Hash::<D>::of(&key);
        let mut path: Path<'_, D> = Path::new(&key_hash);
        let (node, new) = self.link.node().insert(&mut path, hash_leaf(val));
        Self::new(Link::new(node), self.len + usize::from(new))
    }

    /// Inserts all key/value pairs into the map, creating a new map.
    pub fn extend(&self, iter: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut here = self.clone();

        for (key, val) in iter {
            let key_hash = Hash::<D>::of(&key);
            let mut path: Path<'_, D> = Path::new(&key_hash);
            let (node, new) = here.link.node().insert(&mut path, hash_leaf(val));
            here = Self::new(Link::new(node), here.len + usize::from(new));
        }

        here
    }
}

// If updating this function, also update `hash_empty` in crypto crate
/// Compute the hash for an empty leaf using a given Digest algorithm.
#[allow(dead_code)]
pub(crate) fn hash_empty<D: SupportedDigest>() -> Hash<D> {
    hash_leaf(())
}

// No associated function exists in crypto crate today, but in the event that one exists
// update this function if updating the other
/// Hashes a leaf node. See [Map] docs for more detail.
pub(crate) fn hash_leaf<D, V>(value: V) -> Hash<D>
where
    D: SupportedDigest,
    V: VisitBytes,
{
    Hash::of(&(0b0, value))
}

// If updating this function, also update `hash_branch` in crypto crate
/// Hashes a branch node. See [Map] docs for more detail.
pub(crate) fn hash_branch<D>(lhs: &Hash<D>, rhs: &Hash<D>) -> Hash<D>
where
    D: SupportedDigest,
{
    Hash::of((0b1, lhs, rhs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use warg_crypto::hash::Sha256;
    #[test]
    fn empty_map() {
        let map: Map<Sha256, &str, &str> = Map::default();
        assert_eq!(Sha256::empty_tree_hash(256), map.link.hash());
    }
}
