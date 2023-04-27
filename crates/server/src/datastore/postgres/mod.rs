use self::models::{
    NewCheckpoint, NewSource, ParsedText, RecordStatus, Source, SourceKind, Text, TextRef,
};
use super::{DataStore, DataStoreError, InitialLeaf, OperatorLogEntry, PackageLogEntry};
use crate::datastore::postgres::models::{Checkpoint, NewLog, NewRecord};
use anyhow::Result;
use diesel::{prelude::*, result::DatabaseErrorKind};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    scoped_futures::ScopedFutureExt,
    AsyncConnection, AsyncPgConnection, RunQueryDsl,
};
use diesel_json::Json;
use futures::{Stream, StreamExt};
use std::{collections::HashSet, pin::Pin};
use warg_api::content::{ContentSource, ContentSourceKind};
use warg_crypto::{
    hash::DynHash,
    signing::{KeyID, Signature},
    Decode,
};
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, RecordId},
    ProtoEnvelope, Record as _, SerdeEnvelope, Validator,
};

mod models;
mod schema;

async fn get_records<R: Decode>(
    conn: &mut AsyncPgConnection,
    log_id: i32,
    root: &DynHash,
    since: Option<&RecordId>,
) -> Result<Vec<ProtoEnvelope<R>>, DataStoreError> {
    let checkpoint_id = schema::checkpoints::table
        .select(schema::checkpoints::id)
        .filter(schema::checkpoints::checkpoint_id.eq(TextRef(root)))
        .first::<i32>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::CheckpointNotFound(root.clone()))?;

    let mut query = schema::records::table
        .into_boxed()
        .select((schema::records::record_id, schema::records::content))
        .order_by(schema::records::id.asc())
        .filter(
            schema::records::log_id
                .eq(log_id)
                .and(schema::records::checkpoint_id.le(checkpoint_id))
                .and(schema::records::status.eq(RecordStatus::Validated)),
        );

    if let Some(since) = since {
        let record_id = schema::records::table
            .select(schema::records::id)
            .filter(schema::records::record_id.eq(TextRef(since)))
            .first::<i32>(conn)
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::RecordNotFound(since.clone()))?;

        query = query.filter(schema::records::id.gt(record_id));
    }

    query
        .load::<(ParsedText<DynHash>, Vec<u8>)>(conn)
        .await?
        .into_iter()
        .map(|(record_id, c)| {
            ProtoEnvelope::from_protobuf(c).map_err(|e| DataStoreError::InvalidRecordContents {
                record_id: record_id.0.into(),
                message: e.to_string(),
            })
        })
        .collect::<Result<_, _>>()
}

async fn insert_record<V>(
    conn: &mut AsyncPgConnection,
    log_id: &LogId,
    name: Option<&str>,
    record_id: &RecordId,
    record: &ProtoEnvelope<V::Record>,
    sources: &[ContentSource],
) -> Result<(), DataStoreError>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    conn.transaction::<_, DataStoreError, _>(|conn| {
        async move {
            // Unfortunately, this cannot be done with an ON CONFLICT DO NOTHING clause as
            // data cannot be returned; so just do a query for the log id and insert if it doesn't exist.
            let log_id = match schema::logs::table
                .select(schema::logs::id)
                .filter(schema::logs::log_id.eq(TextRef(log_id)))
                .first::<i32>(conn)
                .await
                .optional()?
            {
                Some(id) => id,
                None => diesel::insert_into(schema::logs::table)
                    .values(NewLog {
                        log_id: TextRef(log_id),
                        name,
                        validator: &Json(V::default()),
                    })
                    .returning(schema::logs::id)
                    .get_result::<i32>(conn)
                    .await
                    .map_err(|e| match e {
                        diesel::result::Error::DatabaseError(
                            DatabaseErrorKind::UniqueViolation,
                            _,
                        ) => DataStoreError::Conflict,
                        e => e.into(),
                    })?,
            };

            let id = diesel::insert_into(schema::records::table)
                .values(NewRecord {
                    log_id,
                    record_id: TextRef(record_id),
                    content: &record.to_protobuf(),
                })
                .returning(schema::records::id)
                .get_result::<i32>(conn)
                .await
                .map_err(|e| match e {
                    diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                        DataStoreError::Conflict
                    }
                    e => e.into(),
                })?;

            diesel::insert_into(schema::sources::table)
                .values(
                    &sources
                        .iter()
                        .map(|s| {
                            let (kind, url) = match &s.kind {
                                ContentSourceKind::HttpAnonymous { url } => {
                                    (SourceKind::Http, Some(url.as_str()))
                                }
                            };
                            NewSource {
                                record_id: id,
                                digest: TextRef(&s.digest),
                                kind,
                                url,
                            }
                        })
                        .collect::<Vec<_>>(),
                )
                .execute(conn)
                .await?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
}

async fn reject_record(
    conn: &mut AsyncPgConnection,
    log_id: i32,
    record_id: &RecordId,
    reason: &str,
) -> Result<(), DataStoreError> {
    let count = diesel::update(schema::records::table)
        .filter(
            schema::records::record_id
                .eq(TextRef(record_id))
                .and(schema::records::log_id.eq(log_id))
                .and(schema::records::status.eq(RecordStatus::Pending)),
        )
        .set((
            schema::records::status.eq(RecordStatus::Rejected),
            schema::records::reason.eq(reason),
        ))
        .execute(conn)
        .await?;

    if count != 1 {
        return Err(DataStoreError::RecordNotFound(record_id.clone()));
    }

    Ok(())
}

async fn validate_record<V>(
    conn: &mut AsyncPgConnection,
    log_id: i32,
    record_id: &RecordId,
) -> Result<(), DataStoreError>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    conn.transaction::<_, DataStoreError, _>(|conn| {
        async move {
            // Get the record content and validator
            let (id, content, mut validator) = schema::records::table
                .inner_join(schema::logs::table)
                .select((
                    schema::records::id,
                    schema::records::content,
                    schema::logs::validator,
                ))
                .filter(
                    schema::records::record_id
                        .eq(TextRef(record_id))
                        .and(schema::records::log_id.eq(log_id))
                        .and(schema::records::status.eq(RecordStatus::Pending)),
                )
                .for_update()
                .first::<(i32, Vec<u8>, Json<V>)>(conn)
                .await
                .optional()?
                .ok_or_else(|| DataStoreError::RecordNotPending(record_id.clone()))?;

            let record = ProtoEnvelope::<V::Record>::from_protobuf(content).map_err(|e| {
                DataStoreError::InvalidRecordContents {
                    record_id: record_id.clone(),
                    message: e.to_string(),
                }
            })?;

            let sources: Vec<_> = schema::sources::table
                .select(schema::sources::digest)
                .filter(schema::sources::record_id.eq(id))
                .load::<ParsedText<DynHash>>(conn)
                .await?
                .into_iter()
                .collect();

            let needed = record.as_ref().contents();
            let provided = sources.iter().map(|s| &s.0).collect::<HashSet<_>>();

            if let Some(missing) = needed.difference(&provided).next() {
                return Err(DataStoreError::Rejection(format!(
                    "a content source for digest `{missing}` was not provided"
                )));
            }

            if let Some(extra) = provided.difference(&needed).next() {
                return Err(DataStoreError::Rejection(format!(
                    "a content source for digest `{extra}` was provided but not needed",
                )));
            }

            // Validate the record
            validator.validate(&record).map_err(Into::into)?;

            // Store the updated validation state
            diesel::update(schema::logs::table)
                .filter(schema::logs::id.eq(log_id))
                .set(schema::logs::validator.eq(validator))
                .execute(conn)
                .await?;

            // Finally, mark the record as validated
            diesel::update(schema::records::table)
                .filter(schema::records::id.eq(id))
                .set(schema::records::status.eq(RecordStatus::Validated))
                .execute(conn)
                .await?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
}

async fn get_record<V>(
    conn: &mut AsyncPgConnection,
    log_id: &LogId,
    record_id: &RecordId,
) -> Result<
    (
        ProtoEnvelope<V::Record>,
        Vec<ContentSource>,
        SerdeEnvelope<MapCheckpoint>,
    ),
    DataStoreError,
>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    let log_id = schema::logs::table
        .select(schema::logs::id)
        .filter(schema::logs::log_id.eq(TextRef(log_id)))
        .first::<i32>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

    let (id, content, log_root, log_length, map_root, key_id, signature) = schema::records::table
        .inner_join(schema::checkpoints::table)
        .select((
            schema::records::id,
            schema::records::content,
            schema::checkpoints::log_root,
            schema::checkpoints::log_length,
            schema::checkpoints::map_root,
            schema::checkpoints::key_id,
            schema::checkpoints::signature,
        ))
        .filter(
            schema::records::record_id
                .eq(TextRef(record_id))
                .and(schema::records::log_id.eq(log_id))
                .and(schema::records::status.eq(RecordStatus::Validated)),
        )
        .first::<(
            i32,
            Vec<u8>,
            ParsedText<DynHash>,
            i64,
            ParsedText<DynHash>,
            Text<KeyID>,
            ParsedText<Signature>,
        )>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

    let sources = schema::sources::table
        .filter(schema::sources::record_id.eq(id))
        .load::<Source>(conn)
        .await?
        .into_iter()
        .map(|s| ContentSource {
            kind: match s.kind {
                SourceKind::Http => ContentSourceKind::HttpAnonymous {
                    url: s.url.unwrap_or_default(),
                },
            },
            digest: s.digest.0,
        })
        .collect::<Vec<_>>();

    Ok((
        ProtoEnvelope::from_protobuf(content).map_err(|e| {
            DataStoreError::InvalidRecordContents {
                record_id: record_id.clone(),
                message: e.to_string(),
            }
        })?,
        sources,
        SerdeEnvelope::from_parts_unchecked(
            MapCheckpoint {
                log_root: log_root.0,
                log_length: log_length as u32,
                map_root: map_root.0,
            },
            key_id.0,
            signature.0,
        ),
    ))
}

pub struct PostgresDataStore(Pool<AsyncPgConnection>);

impl PostgresDataStore {
    pub fn new(url: impl Into<String>) -> Result<Self> {
        let config = AsyncDieselConnectionManager::new(url);
        let pool = Pool::builder(config).build()?;
        Ok(Self(pool))
    }
}

#[axum::async_trait]
impl DataStore for PostgresDataStore {
    async fn get_initial_leaves(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<InitialLeaf, DataStoreError>> + Send>>,
        DataStoreError,
    > {
        // The returned future will keep the connection from the pool until dropped
        let mut conn = self.0.get().await?;

        // This is an unfortunate query that will scan the entire records table
        // and join it with the logs and checkpoints tables.
        // In the future, we should figure out a faster way for the transparency service
        // to create its initial state.
        Ok(Box::pin(
            schema::records::table
                .inner_join(schema::logs::table)
                .left_outer_join(schema::checkpoints::table)
                .select((
                    schema::logs::log_id,
                    schema::records::record_id,
                    schema::checkpoints::checkpoint_id.nullable(),
                ))
                .filter(schema::records::status.eq(RecordStatus::Validated))
                .order_by(schema::records::id)
                .load_stream::<(
                    ParsedText<DynHash>,
                    ParsedText<DynHash>,
                    Option<ParsedText<DynHash>>,
                )>(&mut conn)
                .await?
                .map(|r| {
                    r.map_err(Into::into)
                        .map(|(log_id, record_id, checkpoint_id)| InitialLeaf {
                            leaf: LogLeaf {
                                log_id: log_id.0.into(),
                                record_id: record_id.0.into(),
                            },
                            checkpoint: checkpoint_id.map(|c| c.0),
                        })
                }),
        ))
    }

    async fn store_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        record: &ProtoEnvelope<operator::OperatorRecord>,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        insert_record::<operator::Validator>(conn.as_mut(), log_id, None, record_id, record, &[])
            .await
    }

    async fn reject_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        reject_record(conn.as_mut(), log_id, record_id, reason).await
    }

    async fn validate_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        validate_record::<operator::Validator>(conn.as_mut(), log_id, record_id).await
    }

    async fn store_package_record(
        &self,
        log_id: &LogId,
        name: &str,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        sources: &[ContentSource],
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        insert_record::<package::Validator>(
            conn.as_mut(),
            log_id,
            Some(name),
            record_id,
            record,
            sources,
        )
        .await
    }

    async fn reject_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        reject_record(conn.as_mut(), log_id, record_id, reason).await
    }

    async fn validate_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        match validate_record::<package::Validator>(conn.as_mut(), log_id, record_id).await {
            Ok(()) => Ok(()),
            Err(e) => {
                reject_record(conn.as_mut(), log_id, record_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn store_checkpoint(
        &self,
        checkpoint_id: &DynHash,
        checkpoint: SerdeEnvelope<MapCheckpoint>,
        participants: &[LogLeaf],
    ) -> Result<(), DataStoreError> {
        let participants = participants
            .iter()
            .map(|l| l.record_id.to_string())
            .collect::<Vec<_>>();

        let expected_count = participants.len();
        assert!(expected_count > 0);
        let mut conn = self.0.get().await?;

        conn.transaction::<_, DataStoreError, _>(|conn| {
            async move {
                let MapCheckpoint {
                    log_root,
                    log_length,
                    map_root,
                } = checkpoint.as_ref();

                // Insert the checkpoint
                let id = diesel::insert_into(schema::checkpoints::table)
                    .values(NewCheckpoint {
                        checkpoint_id: TextRef(checkpoint_id),
                        log_root: TextRef(log_root),
                        map_root: TextRef(map_root),
                        log_length: *log_length as i64,
                        key_id: TextRef(checkpoint.key_id()),
                        signature: TextRef(checkpoint.signature()),
                    })
                    .returning(schema::checkpoints::id)
                    .get_result::<i32>(conn)
                    .await?;

                // Update all the participants
                let count = diesel::update(schema::records::table)
                    .filter(schema::records::record_id.eq_any(participants))
                    .set(schema::records::checkpoint_id.eq(id))
                    .execute(conn)
                    .await?;

                assert_eq!(
                    count, expected_count,
                    "failed to checkpoint: failed to update all participants"
                );

                Ok(())
            }
            .scope_boxed()
        })
        .await?;

        Ok(())
    }

    async fn get_latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, DataStoreError> {
        let mut conn = self.0.get().await?;

        let checkpoint = schema::checkpoints::table
            .order_by(schema::checkpoints::id.desc())
            .first::<Checkpoint>(&mut conn)
            .await?;

        Ok(SerdeEnvelope::from_parts_unchecked(
            MapCheckpoint {
                log_root: checkpoint.log_root.0,
                log_length: checkpoint.log_length as u32,
                map_root: checkpoint.map_root.0,
            },
            checkpoint.key_id.0,
            checkpoint.signature.0,
        ))
    }

    async fn get_operator_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        get_records(&mut conn, log_id, root, since).await
    }

    async fn get_package_records(
        &self,
        log_id: &LogId,
        root: &DynHash,
        since: Option<&RecordId>,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, DataStoreError> {
        let mut conn = self.0.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        get_records(&mut conn, log_id, root, since).await
    }

    async fn get_record_status(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<super::RecordStatus, DataStoreError> {
        let mut conn = self.0.get().await?;

        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        let (status, reason, checkpoint_id) = schema::records::table
            .select((
                schema::records::status,
                schema::records::reason,
                schema::records::checkpoint_id.nullable(),
            ))
            .filter(
                schema::records::record_id
                    .eq(TextRef(record_id))
                    .and(schema::records::log_id.eq(log_id)),
            )
            .first::<(RecordStatus, Option<String>, Option<i32>)>(&mut conn)
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

        Ok(match status {
            RecordStatus::Pending => super::RecordStatus::Pending,
            RecordStatus::Rejected => super::RecordStatus::Rejected(reason.unwrap_or_default()),
            RecordStatus::Validated => {
                if checkpoint_id.is_some() {
                    super::RecordStatus::InCheckpoint
                } else {
                    super::RecordStatus::Validated
                }
            }
        })
    }

    async fn get_operator_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<OperatorLogEntry, DataStoreError> {
        let mut conn = self.0.get().await?;
        let (record, _, checkpoint) =
            get_record::<operator::Validator>(conn.as_mut(), log_id, record_id).await?;

        Ok(OperatorLogEntry { record, checkpoint })
    }

    async fn get_package_log_entry(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<PackageLogEntry, DataStoreError> {
        let mut conn = self.0.get().await?;
        let (record, sources, checkpoint) =
            get_record::<package::Validator>(conn.as_mut(), log_id, record_id).await?;

        Ok(PackageLogEntry {
            record,
            sources,
            checkpoint,
        })
    }
}
