use super::{hash::Hash, signing::Signature};

pub struct WithBytes<Contents> {
    pub contents: Contents,
    pub bytes: Vec<u8>,
}

pub struct Envelope<Contents> {
    pub contents: WithBytes<Contents>,
    pub key_id: Hash,
    pub signature: Signature,
}
