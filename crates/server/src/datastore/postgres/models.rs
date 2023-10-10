use super::schema::{checkpoints, contents, interfaces, logs, metadata, records};
use chrono::{DateTime, Utc};
use diesel::{
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    prelude::*,
    serialize::{self, IsNull, ToSql},
    sql_types, AsExpression, FromSqlRow, Insertable,
};
use diesel_json::Json;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Error},
    io::Write,
    str::FromStr,
};
use warg_crypto::{
    hash::AnyHash,
    signing::{KeyID, Signature},
};
use warg_protocol::registry::{LogId, RecordId};

#[derive(Debug, Copy, Clone, Eq, PartialEq, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::datastore::postgres::schema::sql_types::RecordStatus"]
pub enum RecordStatus {
    Pending,
    Rejected,
    Validated,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, diesel_derive_enum::DbEnum)]
#[ExistingTypePath = "crate::datastore::postgres::schema::sql_types::Direction"]
pub enum Direction {
    Import,
    Export,
}

impl FromStr for Direction {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "import" {
            return Ok(Self::Import);
        } else if s == "export" {
            return Ok(Self::Export);
        }
        Err(Error {})
    }
}

#[derive(FromSqlRow, AsExpression, Debug)]
#[diesel(sql_type = sql_types::Text)]
pub struct Text<T>(pub T);

impl<T: From<String>> FromSql<sql_types::Text, Pg> for Text<T> {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        Ok(Self(T::from(String::from_sql(bytes)?)))
    }
}

#[derive(FromSqlRow, AsExpression, Debug)]
#[diesel(sql_type = sql_types::Text)]
pub struct ParsedText<T>(pub T);

impl<T: FromStr> FromSql<sql_types::Text, Pg> for ParsedText<T>
where
    <T as std::str::FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        Ok(Self(T::from_str(&String::from_sql(bytes)?)?))
    }
}

#[derive(FromSqlRow, AsExpression, Debug)]
#[diesel(sql_type = sql_types::Text)]
pub struct TextRef<'a, T>(pub &'a T);

impl<'a, T: std::fmt::Debug + Display> ToSql<sql_types::Text, Pg> for TextRef<'a, T> {
    fn to_sql<'b>(&'b self, out: &'b mut serialize::Output<Pg>) -> serialize::Result {
        write!(out, "{}", self.0)?;
        Ok(IsNull::No)
    }
}

#[derive(Insertable)]
#[diesel(table_name = logs)]
pub struct NewLog<'a, V>
where
    V: Serialize,
{
    pub log_id: TextRef<'a, LogId>,
    pub name: Option<&'a str>,
    pub validator: &'a Json<V>,
}

#[derive(Insertable)]
#[diesel(table_name = records)]
pub struct NewRecord<'a> {
    pub log_id: i32,
    pub record_id: TextRef<'a, RecordId>,
    pub content: &'a [u8],
}

#[derive(Insertable)]
#[diesel(table_name = checkpoints)]
pub struct NewCheckpoint<'a> {
    pub checkpoint_id: TextRef<'a, AnyHash>,
    pub log_root: TextRef<'a, AnyHash>,
    pub log_length: i64,
    pub map_root: TextRef<'a, AnyHash>,
    pub key_id: TextRef<'a, KeyID>,
    pub signature: TextRef<'a, Signature>,
    pub timestamp: i64,
}

// impl Expression for RecordId {
//     type SqlType = Direction;
// }
#[derive(Insertable)]
#[diesel(table_name = interfaces)]
pub struct NewInterface<'a> {
    pub content_id: i32,
    pub direction: &'a Direction,
    pub name: &'a str,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = interfaces)]
pub struct Interface<'a> {
    // pub content_id: ParsedText<AnyHash>,
    pub direction: ParsedText<Direction>,
    pub name: TextRef<'a, String>,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = metadata)]
pub struct Metadata<'a, V>
where
    V: Deserialize<'a>,
{
    pub log_id: TextRef<'a, LogId>,
    pub record_id: TextRef<'a, RecordId>,
    pub data: &'a Json<V>,
}

#[derive(Insertable)]
#[diesel(table_name = metadata)]
pub struct NewMetadata<'a, V>
where
    V: Serialize,
{
    pub log_id: i32,
    pub record_id: i32,
    pub data: &'a Json<V>,
}

#[derive(Queryable)]
#[diesel(table_name = checkpoints)]
pub struct CheckpointData {
    pub id: i32,
    pub checkpoint_id: ParsedText<AnyHash>,
    pub log_root: ParsedText<AnyHash>,
    pub log_length: i64,
    pub map_root: ParsedText<AnyHash>,
    pub key_id: Text<KeyID>,
    pub signature: ParsedText<Signature>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub timestamp: i64,
}

/// Selects only the record content and status
#[derive(Queryable, Selectable)]
#[diesel(table_name = records)]
pub struct RecordContent {
    pub status: RecordStatus,
    pub registry_log_index: Option<i64>,
    pub reason: Option<String>,
    pub content: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = contents)]
pub struct NewContent<'a> {
    pub record_id: i32,
    pub digest: TextRef<'a, AnyHash>,
    pub missing: bool,
}
