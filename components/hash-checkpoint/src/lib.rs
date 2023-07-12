use warg_crypto::{signing::KeyID, Decode, hash::{Hash, Sha256, HashAlgorithm, AnyHash}};
use warg_crypto::signing::signature;
use warg_protocol::{SerdeEnvelope, registry::MapCheckpoint};
use std::str::FromStr;

use bindings::exports::component::hash_checkpoint::hashing::{self, Contents, KeyId, Hashing};
struct Component;


impl bindings::exports::component::hash_checkpoint::hashing::Hashing for Component {
    /// Say hello!
    fn hash_checkpoint(contents: Contents, key_id: KeyId, signature: hashing::Signature) -> String {
        let key = KeyID::from(key_id);
        let sig = signature::Signature::from_str(&signature).unwrap();
        let usable = SerdeEnvelope::from_parts_unchecked(
          MapCheckpoint {
            log_root: AnyHash::from_str(&contents.log_root).unwrap(),
            log_length: contents.log_length,
            map_root: AnyHash::from_str(&contents.map_root).unwrap()
          }, key, sig);
        let root: AnyHash = Hash::<Sha256>::of(usable.as_ref()).into();
        dbg!(root);
        "Hello, World!".to_string()
        
    }
}

bindings::export!(Component);
