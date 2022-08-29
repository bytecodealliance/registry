// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

//! Immutable Map w/ Inclusion Proofs
//!
//! The main type in this module is [`Map`]. It implements an immutable map
//! backed by a sparse [Merkle tree][0], which provides the ability to generate
//! inclusion proofs for its items. This data structure is inspired by
//! the [Revocation Transparency][1] effort.
//!
//! [0]: https://en.wikipedia.org/wiki/Merkle_tree
//! [1]: https://www.links.org/files/RevocationTransparency.pdf

#![allow(clippy::module_inception)]

mod hash;
mod iter;
mod link;
mod map;
mod node;
mod path;
mod proof;

pub use iter::Iter;
pub use map::Map;
pub use proof::Proof;

#[cfg(test)]
mod test {
    use digest::Digest;
    use sha2::Sha256;

    use super::Map;

    #[test]
    fn insert() {
        // Prepare three trees.
        let first = Map::<Sha256, &'static str, &'static str>::default();
        let second = first.insert("foo", "bar");
        let third = second.insert("baz", "bat");

        // Ensure the digests don't match.
        assert_ne!(first.root(), second.root());
        assert_ne!(first.root(), third.root());
        assert_ne!(second.root(), third.root());

        // Ensure the empty tree has the known root.
        assert_eq!(&**first.root(), &Sha256::digest(&[0x00u8]));

        // Check that values returned are correct.
        assert_eq!(first.get(&"foo"), None);
        assert_eq!(second.get(&"baz"), None);
        assert_eq!(second.get(&"foo"), Some(&"bar"));
        assert_eq!(second.get(&"baz"), None);
        assert_eq!(third.get(&"foo"), Some(&"bar"));
        assert_eq!(third.get(&"baz"), Some(&"bat"));
    }

    #[test]
    fn len() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        assert_eq!(first.len(), 0);

        let second = first.insert("foo", "bar");
        assert_eq!(second.len(), 1);

        let third = second.insert("baz", "bat");
        assert_eq!(third.len(), 2);

        let fourth = third.insert("foo", "qux");
        assert_eq!(fourth.len(), 2);
    }

    #[test]
    fn is_empty() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        assert!(first.is_empty());

        let second = first.insert("foo", "bar");
        assert!(!second.is_empty());

        let third = second.insert("baz", "bat");
        assert!(!third.is_empty());
    }

    #[test]
    fn extend() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        let second = first.insert("foo", "bar");
        let third = second.insert("baz", "bat");

        let extended = first.extend([("foo", "bar"), ("baz", "bat")]);
        assert!(!extended.is_empty());
        assert_eq!(extended.len(), 2);
        assert_eq!(extended, third);

        let extended = first.extend([("baz", "bat"), ("foo", "bar")]);
        assert!(!extended.is_empty());
        assert_eq!(extended.len(), 2);
        assert_eq!(extended, third);
    }

    #[test]
    fn replace() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        let second = first.insert("foo", "bar");
        assert_eq!(second.get(&"foo"), Some(&"bar"));
        assert_eq!(second.len(), 1);

        let third = second.insert("foo", "baz");
        assert_eq!(third.get(&"foo"), Some(&"baz"));
        assert_eq!(third.len(), 1);
    }

    #[test]
    fn iter() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        let mut iter = first.iter();
        assert_eq!(iter.next(), None);

        let second = first.insert("foo", "bar");
        let mut iter = second.iter();
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some((&"foo", &"bar")));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);

        let third = second.insert("baz", "bat");
        let mut iter = third.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some((&"foo", &"bar")));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some((&"baz", &"bat")));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);

        let extended = first.extend([("foo", "bar"), ("baz", "bat")]);
        let mut iter = extended.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some((&"foo", &"bar")));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some((&"baz", &"bat")));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);

        // The order of iteration DOES NOT depend on order of insertion.
        let extended = first.extend([("baz", "bat"), ("foo", "bar")]);
        let mut iter = extended.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some((&"foo", &"bar")));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some((&"baz", &"bat")));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn prove() {
        fn check<D: Digest, K: AsRef<[u8]>, V: AsRef<[u8]>>(
            tree: &Map<D, K, V>,
            key: K,
            peers: usize,
        ) {
            let proof = tree.prove(&key).unwrap();
            assert_eq!(proof.peers.len(), peers);
            assert!(proof.verify(tree.root(), &key));
        }

        let first = Map::<Sha256, &'static str, &'static str>::default();
        assert!(first.prove(&"foo").is_none());
        assert!(first.prove(&"baz").is_none());
        assert!(first.prove(&"qux").is_none());

        let second = first.insert("foo", "bar");
        check(&second, "foo", 0);
        assert!(second.prove(&"baz").is_none());
        assert!(second.prove(&"qux").is_none());

        let third = second.insert("baz", "bat");
        check(&third, "foo", 1);
        check(&third, "baz", 1);
        assert!(third.prove(&"qux").is_none());
    }
}
