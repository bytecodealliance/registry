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

mod fork;
mod link;
mod map;
mod node;
mod path;
mod proof;
mod proof_bundle;
mod singleton;

pub use map::Map;
pub use proof::Proof;
pub use proof_bundle::ProofBundle as MapProofBundle;

#[cfg(test)]
mod test {
    use warg_crypto::{
        hash::{AnyHash, Hash, Sha256, SupportedDigest},
        VisitBytes,
    };

    use super::Map;

    #[test]
    fn insert() {
        // Prepare three trees.
        let first = Map::<Sha256, &'static str, &'static str>::default();
        let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        let third = second.insert(AnyHash::from(Hash::<Sha256>::of("baz")), "bat");
        // let second = first.insert("foo", "bar");
        // let third = second.insert("baz", "bat");

        // Ensure the digests don't match.
        assert_ne!(first.root(), second.root());
        assert_ne!(first.root(), third.root());
        assert_ne!(second.root(), third.root());

        // Ensure the empty tree has the known root.
        assert_eq!(first.root().clone(), Sha256::empty_tree_hash(256));
    }

    #[test]
    fn len() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        assert_eq!(first.len(), 0);

        let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        assert_eq!(second.len(), 1);

        let third = second.insert(AnyHash::from(Hash::<Sha256>::of("bar")), "bat");
        assert_eq!(third.len(), 2);

        let fourth = third.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "qux");
        assert_eq!(fourth.len(), 2);
    }

    #[test]
    fn is_empty() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        assert!(first.is_empty());

        // let second = first.insert("foo", "bar");
        let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        assert!(!second.is_empty());

        // let third = second.insert("bar", "bat");
        let third = second.insert(AnyHash::from(Hash::<Sha256>::of("bar")), "bat");
        assert!(!third.is_empty());
    }

    // #[test]
    // fn extend() {
    //     let first = Map::<Sha256, &'static str, &'static str>::default();
    //     let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
    //     let third = second.insert(AnyHash::from(Hash::<Sha256>::of("bar")), "bat");
    //     // let second = first.insert("foo", "bar");
    //     // let third = second.insert("baz", "bat");

    //     // let extended = first.extend([("foo", "bar"), ("baz", "bat")]);
    //     // let extended = first.extend([(Hash::<Sha256>::of("foo"), "bar"), (Hash::<Sha256>::of("bar"), "bat")]);
    //     assert!(!extended.is_empty());
    //     assert_eq!(extended.len(), 2);
    //     assert_eq!(extended, third);

    //     let extended = first.extend([("baz", "bat"), ("foo", "bar")]);
    //     assert!(!extended.is_empty());
    //     assert_eq!(extended.len(), 2);
    //     assert_eq!(extended, third);
    // }

    #[test]
    fn replace() {
        let first = Map::<Sha256, &'static str, &'static str>::default();
        // let second = first.insert("foo", "bar");
        let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        assert_eq!(second.len(), 1);

        // let third = second.insert("foo", "baz");
        let third = second.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "baz");
        assert_eq!(third.len(), 1);

        // Ensure the digests don't match.
        assert_ne!(first.root(), second.root());
        assert_ne!(first.root(), third.root());
        assert_ne!(second.root(), third.root());
    }

    #[test]
    fn prove() {
        fn check<
            D: SupportedDigest + std::fmt::Debug,
            K: VisitBytes + Clone + std::fmt::Debug,
            V: VisitBytes,
        >(
            tree: &Map<D, K, V>,
            key: AnyHash,
            value: V,
        ) {
            let proof = tree.prove(key.clone()).unwrap();
            assert_eq!(tree.root().clone(), proof.evaluate(key, &value));
        }

        let first = Map::<Sha256, &'static str, &'static str>::default();
        // let proof = first.prove(AnyHash::from(Hash::of("foo"))
        assert!(first
            .prove(AnyHash::from(Hash::<Sha256>::of("foo")))
            .is_none());
        assert!(first
            .prove(AnyHash::from(Hash::<Sha256>::of("baz")))
            .is_none());
        assert!(first
            .prove(AnyHash::from(Hash::<Sha256>::of("qux")))
            .is_none());

        let second = first.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        check(&second, AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        assert!(second
            .prove(AnyHash::from(Hash::<Sha256>::of("baz")))
            .is_none());
        assert!(second
            .prove(AnyHash::from(Hash::<Sha256>::of("qux")))
            .is_none());
        let third = second.insert(AnyHash::from(Hash::<Sha256>::of("bar")), "bat");
        check(&third, AnyHash::from(Hash::<Sha256>::of("foo")), "bar");
        check(&third, AnyHash::from(Hash::<Sha256>::of("bar")), "bat");
        assert!(third
            .prove(AnyHash::from(Hash::<Sha256>::of("qux")))
            .is_none());

        let fourth = third.insert(AnyHash::from(Hash::<Sha256>::of("foo")), "qux");
        check(&fourth, AnyHash::from(Hash::<Sha256>::of("foo")), "qux");
    }
}
