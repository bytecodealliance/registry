use self::models::{
    Checkpoint, NewCheckpoint, NewContent, NewLog, NewRecord, ParsedText, RecordContent,
    RecordStatus, TextRef,
};
use super::{DataStore, DataStoreError, Record};
use anyhow::{anyhow, Result};
use diesel::{prelude::*, result::DatabaseErrorKind};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    scoped_futures::ScopedFutureExt,
    AsyncConnection, AsyncPgConnection, RunQueryDsl,
};
use diesel_json::Json;
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use futures::{Stream, StreamExt};
use secrecy::{ExposeSecret, SecretString};
use std::{collections::HashSet, pin::Pin};
use warg_crypto::{hash::AnyHash, Decode, Signable};
use warg_protocol::{
    operator,
    package::{self, PackageEntry},
    registry::{LogId, LogLeaf, MapCheckpoint, PackageId, RecordId},
    ProtoEnvelope, Record as _, SerdeEnvelope, Validator,
};

mod models;
mod schema;

async fn get_records<R: Decode>(
    conn: &mut AsyncPgConnection,
    log_id: i32,
    checkpoint_id: &AnyHash,
    since: Option<&RecordId>,
    limit: i64,
) -> Result<Vec<ProtoEnvelope<R>>, DataStoreError> {
    let checkpoint_length = schema::checkpoints::table
        .select(schema::checkpoints::log_length)
        .filter(schema::checkpoints::checkpoint_id.eq(TextRef(checkpoint_id)))
        .first::<i64>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::CheckpointNotFound(checkpoint_id.clone()))?;

    let mut query = schema::records::table
        .into_boxed()
        .select((schema::records::record_id, schema::records::content))
        .order_by(schema::records::id.asc())
        .limit(limit)
        .filter(
            schema::records::log_id
                .eq(log_id)
                .and(schema::records::registry_log_index.lt(checkpoint_length))
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
        .load::<(ParsedText<AnyHash>, Vec<u8>)>(conn)
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
    missing: &HashSet<&AnyHash>,
) -> Result<(), DataStoreError>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    let contents = record.as_ref().contents();
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

            let record_id = diesel::insert_into(schema::records::table)
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

            if !contents.is_empty() {
                diesel::insert_into(schema::contents::table)
                    .values(
                        &contents
                            .iter()
                            .map(|s| NewContent {
                                record_id,
                                digest: TextRef(s),
                                missing: missing.contains(s),
                            })
                            .collect::<Vec<_>>(),
                    )
                    .execute(conn)
                    .await?;
            }

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

async fn commit_record<V>(
    conn: &mut AsyncPgConnection,
    log_id: i32,
    record_id: &RecordId,
    registry_log_index: u64,
) -> Result<(), DataStoreError>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    let registry_log_index: i64 = registry_log_index.try_into().unwrap();
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
                .set((
                    schema::records::status.eq(RecordStatus::Validated),
                    schema::records::registry_log_index.eq(Some(registry_log_index)),
                ))
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
) -> Result<Record<V::Record>, DataStoreError>
where
    V: Validator + 'static,
    <V as Validator>::Error: ToString + Send + Sync,
    DataStoreError: From<<V as Validator>::Error>,
{
    let checkpoint = schema::checkpoints::table
        .order_by(schema::checkpoints::id.desc())
        .first::<Checkpoint>(conn)
        .await?;

    let log_id = schema::logs::table
        .select(schema::logs::id)
        .filter(schema::logs::log_id.eq(TextRef(log_id)))
        .first::<i32>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

    let record = schema::records::table
        .select(RecordContent::as_select())
        .filter(
            schema::records::record_id
                .eq(TextRef(record_id))
                .and(schema::records::log_id.eq(log_id)),
        )
        .first::<RecordContent>(conn)
        .await
        .optional()?
        .ok_or_else(|| DataStoreError::RecordNotFound(record_id.clone()))?;

    Ok(Record {
        status: match record.status {
            RecordStatus::Pending => {
                // Get the missing content
                let missing = schema::contents::table
                    .inner_join(schema::records::table)
                    .select(schema::contents::digest)
                    .filter(
                        schema::records::record_id
                            .eq(TextRef(record_id))
                            .and(schema::contents::missing.eq(true)),
                    )
                    .load::<ParsedText<AnyHash>>(conn)
                    .await?;

                if missing.is_empty() {
                    super::RecordStatus::Pending
                } else {
                    super::RecordStatus::MissingContent(missing.into_iter().map(|d| d.0).collect())
                }
            }
            RecordStatus::Validated => {
                if record.registry_log_index.unwrap() < checkpoint.log_length {
                    super::RecordStatus::Published
                } else {
                    super::RecordStatus::Validated
                }
            }
            RecordStatus::Rejected => {
                super::RecordStatus::Rejected(record.reason.unwrap_or_default())
            }
        },
        envelope: ProtoEnvelope::from_protobuf(record.content).map_err(|e| {
            DataStoreError::InvalidRecordContents {
                record_id: record_id.clone(),
                message: e.to_string(),
            }
        })?,
        registry_log_index: record.registry_log_index.map(|idx| idx.try_into().unwrap()),
    })
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/datastore/postgres/migrations");

pub struct PostgresDataStore {
    url: SecretString,
    pool: Pool<AsyncPgConnection>,
}

impl PostgresDataStore {
    pub fn new(url: SecretString) -> Result<Self> {
        let config = AsyncDieselConnectionManager::new(url.expose_secret());
        let pool = Pool::builder(config).build()?;
        Ok(Self { url, pool })
    }

    pub async fn run_pending_migrations(&self) -> Result<()> {
        let mut conn = diesel::pg::PgConnection::establish(self.url.expose_secret())?;

        // Send migration output to tracing::info
        struct TracingWriter;
        impl std::io::Write for TracingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                tracing::info!("{}", String::from_utf8_lossy(buf).trim_end());
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut harness =
            HarnessWithOutput::new(&mut conn, std::io::LineWriter::new(TracingWriter));

        harness
            .run_pending_migrations(MIGRATIONS)
            .map_err(|err| anyhow!("migrations failed: {err:?}"))?;

        Ok(())
    }
}

#[axum::async_trait]
impl DataStore for PostgresDataStore {
    async fn get_all_checkpoints(
        &self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<MapCheckpoint, DataStoreError>> + Send>>,
        DataStoreError,
    > {
        // The returned future will keep the connection from the pool until dropped
        let mut conn = self.pool.get().await?;

        Ok(schema::checkpoints::table
            .order_by(schema::checkpoints::id.desc())
            .load_stream::<Checkpoint>(&mut conn)
            .await?
            .map(|checkpoint| -> Result<MapCheckpoint, DataStoreError> {
                let checkpoint = checkpoint?;
                Ok(MapCheckpoint {
                    log_root: checkpoint.log_root.0,
                    log_length: checkpoint.log_length as u32,
                    map_root: checkpoint.map_root.0,
                })
            })
            .boxed())
    }

    async fn get_all_validated_records(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<LogLeaf, DataStoreError>> + Send>>, DataStoreError>
    {
        // The returned future will keep the connection from the pool until dropped
        let mut conn = self.pool.get().await?;

        // This is an unfortunate query that will scan the entire records table
        // and join it with the logs table.
        // In the future, we should figure out a faster way for the transparency service
        // to create its initial state.
        Ok(Box::pin(
            schema::records::table
                .inner_join(schema::logs::table)
                .select((schema::logs::log_id, schema::records::record_id))
                .filter(schema::records::status.eq(RecordStatus::Validated))
                .order_by(schema::records::id)
                .load_stream::<(ParsedText<AnyHash>, ParsedText<AnyHash>)>(&mut conn)
                .await?
                .map(|r| {
                    r.map_err(Into::into).map(|(log_id, record_id)| LogLeaf {
                        log_id: log_id.0.into(),
                        record_id: record_id.0.into(),
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
        let mut conn = self.pool.get().await?;
        insert_record::<operator::LogState>(
            conn.as_mut(),
            log_id,
            None,
            record_id,
            record,
            &Default::default(),
        )
        .await
    }

    async fn reject_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        reject_record(conn.as_mut(), log_id, record_id, reason).await
    }

    async fn commit_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_log_index: u64,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        match commit_record::<operator::LogState>(
            conn.as_mut(),
            log_id,
            record_id,
            registry_log_index,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                reject_record(conn.as_mut(), log_id, record_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn store_package_record(
        &self,
        log_id: &LogId,
        package_id: &PackageId,
        record_id: &RecordId,
        record: &ProtoEnvelope<package::PackageRecord>,
        missing: &HashSet<&AnyHash>,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;
        insert_record::<package::LogState>(
            conn.as_mut(),
            log_id,
            Some(package_id.as_ref()),
            record_id,
            record,
            missing,
        )
        .await
    }

    async fn reject_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        reason: &str,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        reject_record(conn.as_mut(), log_id, record_id, reason).await
    }

    async fn commit_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        registry_log_index: u64,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        match commit_record::<package::LogState>(
            conn.as_mut(),
            log_id,
            record_id,
            registry_log_index,
        )
        .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                reject_record(conn.as_mut(), log_id, record_id, &e.to_string()).await?;
                Err(e)
            }
        }
    }

    async fn is_content_missing(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError> {
        let mut conn = self.pool.get().await?;
        schema::contents::table
            .inner_join(schema::records::table)
            .inner_join(schema::logs::table.on(schema::logs::id.eq(schema::records::log_id)))
            .select(schema::contents::missing)
            .filter(
                schema::records::status
                    .eq(RecordStatus::Pending)
                    .and(schema::logs::log_id.eq(TextRef(log_id)))
                    .and(schema::records::record_id.eq(TextRef(record_id)))
                    .and(schema::contents::digest.eq(TextRef(digest))),
            )
            .first::<bool>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::RecordNotPending(record_id.clone()))
    }

    async fn set_content_present(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<bool, DataStoreError> {
        let mut conn = self.pool.get().await?;
        conn.transaction::<_, DataStoreError, _>(|conn| {
            // Diesel currently doesn't support joins for updates
            // See: https://github.com/diesel-rs/diesel/issues/1478
            // So we select the record id first and then update the content
            async move {
                let record_id = schema::records::table
                    .inner_join(schema::logs::table)
                    .select(schema::records::id)
                    .filter(
                        schema::records::status
                            .eq(RecordStatus::Pending)
                            .and(schema::logs::log_id.eq(TextRef(log_id)))
                            .and(schema::records::record_id.eq(TextRef(record_id))),
                    )
                    .first::<i32>(conn.as_mut())
                    .await
                    .optional()?
                    .ok_or_else(|| DataStoreError::RecordNotPending(record_id.clone()))?;

                // If the row was already updated, return false since this update
                // didn't change anything
                if diesel::update(schema::contents::table)
                    .filter(
                        schema::contents::record_id
                            .eq(record_id)
                            .and(schema::contents::digest.eq(TextRef(digest))),
                    )
                    .set(schema::contents::missing.eq(false))
                    .execute(conn.as_mut())
                    .await?
                    == 0
                {
                    return Ok(false);
                }

                // Finally, check if all contents are present; if so, return true
                // to indicate that this record is ready to be processed
                let missing = schema::contents::table
                    .select(schema::contents::id)
                    .filter(
                        schema::contents::record_id
                            .eq(record_id)
                            .and(schema::contents::missing.eq(true)),
                    )
                    .first::<i32>(conn.as_mut())
                    .await
                    .optional()?;

                Ok(missing.is_none())
            }
            .scope_boxed()
        })
        .await
    }

    async fn store_checkpoint(
        &self,
        checkpoint_id: &AnyHash,
        checkpoint: SerdeEnvelope<MapCheckpoint>,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;

        conn.transaction::<_, DataStoreError, _>(|conn| {
            async move {
                let MapCheckpoint {
                    log_root,
                    log_length,
                    map_root,
                } = checkpoint.as_ref();

                // Insert the checkpoint
                diesel::insert_into(schema::checkpoints::table)
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

                Ok(())
            }
            .scope_boxed()
        })
        .await?;

        Ok(())
    }

    async fn get_latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, DataStoreError> {
        let mut conn = self.pool.get().await?;

        let checkpoint = schema::checkpoints::table
            .order_by(schema::checkpoints::id.desc())
            .first::<Checkpoint>(&mut conn)
            .await?;

        let log_length = checkpoint.log_length.try_into().unwrap();

        Ok(SerdeEnvelope::from_parts_unchecked(
            MapCheckpoint {
                log_root: checkpoint.log_root.0,
                log_length,
                map_root: checkpoint.map_root.0,
            },
            checkpoint.key_id.0,
            checkpoint.signature.0,
        ))
    }

    async fn get_operator_records(
        &self,
        log_id: &LogId,
        checkpoint_id: &AnyHash,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<ProtoEnvelope<operator::OperatorRecord>>, DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        get_records(&mut conn, log_id, checkpoint_id, since, limit as i64).await
    }

    async fn get_package_records(
        &self,
        log_id: &LogId,
        checkpoint_id: &AnyHash,
        since: Option<&RecordId>,
        limit: u16,
    ) -> Result<Vec<ProtoEnvelope<package::PackageRecord>>, DataStoreError> {
        let mut conn = self.pool.get().await?;
        let log_id = schema::logs::table
            .select(schema::logs::id)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<i32>(conn.as_mut())
            .await
            .optional()?
            .ok_or_else(|| DataStoreError::LogNotFound(log_id.clone()))?;

        get_records(&mut conn, log_id, checkpoint_id, since, limit as i64).await
    }

    async fn get_operator_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<Record<operator::OperatorRecord>, DataStoreError> {
        let mut conn = self.pool.get().await?;
        get_record::<operator::LogState>(conn.as_mut(), log_id, record_id).await
    }

    async fn get_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<Record<package::PackageRecord>, DataStoreError> {
        let mut conn = self.pool.get().await?;
        get_record::<package::LogState>(conn.as_mut(), log_id, record_id).await
    }

    async fn verify_package_record_signature(
        &self,
        log_id: &LogId,
        record: &ProtoEnvelope<package::PackageRecord>,
    ) -> Result<(), DataStoreError> {
        let mut conn = self.pool.get().await?;

        let validator = schema::logs::table
            .select(schema::logs::validator)
            .filter(schema::logs::log_id.eq(TextRef(log_id)))
            .first::<Json<package::LogState>>(&mut conn)
            .await
            .optional()?;

        let key = match validator
            .as_ref()
            .and_then(|v| v.public_key(record.key_id()))
        {
            Some(key) => key,
            None => match record.as_ref().entries.get(0) {
                Some(PackageEntry::Init { key, .. }) => key,
                _ => return Err(DataStoreError::UnknownKey(record.key_id().clone())),
            },
        };

        package::PackageRecord::verify(key, record.content_bytes(), record.signature())
            .map_err(|_| DataStoreError::SignatureVerificationFailed)
    }

    #[cfg(feature = "debug")]
    async fn debug_list_package_ids(&self) -> anyhow::Result<Vec<PackageId>> {
        let mut conn = self.pool.get().await?;
        let names = schema::logs::table
            .select(schema::logs::name)
            .load::<Option<String>>(&mut conn)
            .await?
            .into_iter()
            .flatten()
            .filter_map(|name| name.parse().ok())
            .collect();
        Ok(names)
    }
}
