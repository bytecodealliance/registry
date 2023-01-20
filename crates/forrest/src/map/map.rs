use core::borrow::Borrow;
use core::fmt::{Debug, Formatter, LowerHex};

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use warg_crypto::hash::{Hash, Output, SupportedDigest};

use super::iter::Iter;
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
/// assert_eq!(c.get("baz"), Some(&"bat"));
/// assert_eq!(c.get("lux"), None);
///
/// let proof = c.prove("foo").unwrap();
/// assert!(proof.verify(c.root(), "foo"));
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
pub struct Map<D: SupportedDigest, K, V>(Link<D, K, V>, usize);

impl<D: SupportedDigest, K: AsRef<[u8]>, V: AsRef<[u8]>> Clone for Map<D, K, V> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<D: SupportedDigest, K: AsRef<[u8]>, V: AsRef<[u8]>> Default for Map<D, K, V> {
    fn default() -> Self {
        Self(Link::default(), 0)
    }
}

impl<D: SupportedDigest, K, V> Eq for Map<D, K, V> {}
impl<D: SupportedDigest, K, V> PartialEq for Map<D, K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl<D: SupportedDigest, K, V> core::hash::Hash for Map<D, K, V> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state);
    }
}

impl<D: SupportedDigest, K, V> Debug for Map<D, K, V>
where
    Output<D>: LowerHex,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("Map({:?})", &self.0.hash))
    }
}

impl<D: SupportedDigest, K, V> Map<D, K, V> {
    /// The hash of the root of the tree.
    ///
    /// This uniquely identifies the map and its contents.
    pub fn root(&self) -> &Hash<D> {
        &self.0.hash
    }

    /// The number of items in the map.
    pub fn len(&self) -> usize {
        self.1
    }

    /// Whether or not the map is empty.
    pub fn is_empty(&self) -> bool {
        self.1 == 0
    }

    /// Iterates over all the keys and values in the map.
    pub fn iter(&self) -> Iter<'_, D, K, V> {
        Iter::new(&self.0.node, self.1)
    }

    /// Gets the value for a given key.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: AsRef<[u8]>,
    {
        self.0.node.get(Path::from(&key))
    }

    /// Gets the value for a given key and a proof of its presence in this map.
    pub fn prove<Q: ?Sized>(&self, key: &Q) -> Option<Proof<D, &Hash<D>, V>>
    where
        K: Borrow<Q>,
        Q: AsRef<[u8]>,
    {
        self.0.node.prove(Path::from(&key))
    }
}

impl<D: SupportedDigest, K: AsRef<[u8]>, V: AsRef<[u8]>> Map<D, K, V> {
    /// Insert a value into the map, creating a new map.
    ///
    /// This replaces any existing items with the same key.
    pub fn insert(&self, key: K, val: V) -> Self {
        let mut path = Path::from(&key);
        let (node, new) = self.0.node.insert(&mut path, (key, val));
        Self(path.link(node), self.1 + usize::from(new))
    }

    /// Inserts all key/value pairs into the map, creating a new map.
    pub fn extend(&self, iter: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut here = Self(self.0.clone(), self.1);

        for (key, val) in iter {
            let mut path = Path::from(&key);
            let (node, new) = here.0.node.insert(&mut path, (key, val));
            here = Self(path.link(node), here.1 + usize::from(new));
        }

        here
    }
}

impl<D: SupportedDigest, K: Serialize, V: Serialize> Serialize for Map<D, K, V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.len()))?;

        for (k, v) in self.iter() {
            map.serialize_entry(k, v)?;
        }

        map.end()
    }
}

impl<'de, D, K, V> Deserialize<'de> for Map<D, K, V>
where
    D: SupportedDigest,
    K: AsRef<[u8]> + Deserialize<'de>,
    V: AsRef<[u8]> + Deserialize<'de>,
{
    fn deserialize<X: Deserializer<'de>>(deserializer: X) -> Result<Self, X::Error> {
        struct MapVisitor<D: SupportedDigest, K, V>(Map<D, K, V>);

        impl<'a, D, K, V> Visitor<'a> for MapVisitor<D, K, V>
        where
            D: SupportedDigest,
            K: AsRef<[u8]> + Deserialize<'a>,
            V: AsRef<[u8]> + Deserialize<'a>,
        {
            type Value = Map<D, K, V>;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> alloc::fmt::Result {
                formatter.write_str("map")
            }

            fn visit_map<A: MapAccess<'a>>(mut self, mut map: A) -> Result<Self::Value, A::Error> {
                while let Some((k, v)) = map.next_entry()? {
                    self.0 = self.0.insert(k, v);
                }

                Ok(self.0)
            }
        }

        deserializer.deserialize_map(MapVisitor(Map::default()))
    }
}

#[test]
#[cfg(test)]
fn test() {
    use alloc::{string::String, vec::Vec};
    use warg_crypto::hash::Sha256;

    let a = Map::<Sha256, &str, &str>::default();
    let b = a.insert("foo", "bar");
    let c = b.insert("baz", "bat");

    let mut buf = Vec::new();
    ciborium::ser::into_writer(&c, &mut buf).unwrap();

    for byte in &buf {
        std::eprint!("{:02x}", byte);
    }
    std::eprintln!();

    let map: Map<Sha256, String, String> = ciborium::de::from_reader(&buf[..]).unwrap();
    assert_eq!(c.root(), map.root());
}
