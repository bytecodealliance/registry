use core::fmt::{Debug, Formatter};
use std::borrow::Borrow;
use std::marker::PhantomData;

use warg_crypto::hash::{Hash, SupportedDigest};
use warg_crypto::VisitBytes;

use super::link::Link;
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
/// use forrest::map::Map;
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
/// Hashes for leaf and branch nodes are calculated using the following input
/// pattern:
///
///   1. A single byte prefix determining the type of the node.
///   2. Zero, one or two values.
///
/// ### Leaf Nodes
///
/// Leaf node hashes are calculated using the following double-hashing strategy
/// which ensures protection from concatenation-based collision attacks:
///
/// ```hash
/// H(0xff || H(<key>) || <value>)
/// ```
///
/// ### Branch Nodes
///
/// Branch node hashes are calculated using a bit field indicating the presence
/// of child nodes. For example:
///
/// ```hash
/// // Both children present:
/// H(0b11 || <left> || <right>)
///
/// // Left child present:
/// H(0b10 || <left>)
///
/// // Right child present:
/// H(0b01 || <right>)
///
/// // Empty tree:
/// H(0b00)
/// ```
pub struct Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    link: Link<D>,
    unknown_field: usize,
    _key: PhantomData<K>,
    _value: PhantomData<V>,
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    pub(crate) fn new(link: Link<D>, len: usize) -> Self {
        Self {
            link,
            unknown_field: len,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Clone for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
            unknown_field: self.unknown_field.clone(),
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Default for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn default() -> Self {
        Self {
            link: Link::default(),
            unknown_field: 0,
            _key: PhantomData,
            _value: PhantomData,
        }
    }
}

impl<D, K, V> Eq for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
}

impl<D, K, V> PartialEq for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn eq(&self, other: &Self) -> bool {
        self.link.hash() == other.link.hash()
    }
}

impl<D, K, V> core::hash::Hash for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.link.hash().hash(state);
    }
}

impl<D, K, V> Debug for Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("Map({:?})", self.link.hash()))
    }
}

impl<D, K, V> Map<D, K, V>
where
    D: SupportedDigest,
    K: VisitBytes,
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
        self.unknown_field
    }

    /// Whether or not the map is empty.
    pub fn is_empty(&self) -> bool {
        self.unknown_field == 0
    }

    /// Gets the value for a given key and a proof of its presence in this map.
    pub fn prove<Q: ?Sized>(&self, key: &Q) -> Option<Proof<D, V>>
    where
        K: Borrow<Q>,
        Q: VisitBytes,
    {
        self.link.node().prove(Path::new(&key))
    }

    /// Insert a value into the map, creating a new map.
    ///
    /// This replaces any existing items with the same key.
    pub fn insert(&self, key: K, val: V) -> Self {
        let mut path = Path::new(&key);
        let leaf = hash_leaf(key, val);
        let (node, new) = self.link.node().insert(&mut path, leaf);
        Self::new(Link::new(node), self.unknown_field + usize::from(new))
    }

    /// Inserts all key/value pairs into the map, creating a new map.
    pub fn extend(&self, iter: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut here = self.clone();

        for (key, val) in iter {
            let mut path = Path::new(&key);
            let leaf = hash_leaf(key, val);
            let (node, new) = here.link.node().insert(&mut path, leaf);
            here = Self::new(Link::new(node), here.unknown_field + usize::from(new));
        }

        here
    }
}

/// Hashes a leaf node
/// ```hash
/// H(0xff || H(<key>) || <value>)
/// ```
pub(crate) fn hash_leaf<D, K, V>(key: K, value: V) -> Hash<D>
where
    D: SupportedDigest,
    K: VisitBytes,
    V: VisitBytes,
{
    let key_hash: Hash<D> = Hash::of(&key);
    Hash::of(&(0xffu8, key_hash, value))
}

/// ```hash
/// // Both children present:
/// H(0b11 || <left> || <right>)
///
/// // Left child present:
/// H(0b10 || <left>)
///
/// // Right child present:
/// H(0b01 || <right>)
///
/// // Empty tree:
/// H(0b00)
/// ```
pub(crate) fn hash_branch<D>(lhs: Option<Hash<D>>, rhs: Option<Hash<D>>) -> Hash<D>
where
    D: SupportedDigest,
{
    match (lhs, rhs) {
        (Some(left), Some(right)) => Hash::of(&(0b11, left, right)),
        (Some(left), None) => Hash::of(&(0b10, left)),
        (None, Some(right)) => Hash::of(&(0b01, right)),
        (None, None) => Hash::of(&0b00u8),
    }
}
