use crate::{protobuf};
use anyhow::{Error};
use prost::Message;
use warg_crypto::{Decode};

pub mod model;

impl Decode for model::Query {
  fn decode(bytes: &[u8]) -> Result<Self, Error> {
      protobuf::Query::decode(bytes)?.try_into()
  }
}

impl TryFrom<protobuf::Query> for model::Query {
  type Error = Error;

  fn try_from(query: protobuf::Query) -> Result<Self, Self::Error> {
      Ok(model::Query {
        names: query.names
      })
  }
}