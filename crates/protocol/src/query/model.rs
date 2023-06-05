use anyhow::Error;

use serde::{Deserialize, Serialize};
use warg_crypto::Decode;

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    /// The hash of the previous operator record envelope
    pub names: Vec::<String>,
}

impl Query {
  pub fn from_protobuf(bytes: Vec<u8>) -> Result<Self, Error> {
    let query = Query::decode(bytes.as_slice());
    query
  }
}