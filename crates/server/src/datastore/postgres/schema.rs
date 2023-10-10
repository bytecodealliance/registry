// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "direction"))]
    pub struct Direction;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "record_status"))]
    pub struct RecordStatus;
}

diesel::table! {
    checkpoints (id) {
        id -> Int4,
        checkpoint_id -> Text,
        log_root -> Text,
        log_length -> Int8,
        map_root -> Text,
        key_id -> Text,
        signature -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        timestamp -> Int8,
    }
}

diesel::table! {
    contents (id) {
        id -> Int4,
        record_id -> Int4,
        digest -> Text,
        missing -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Direction;

    interfaces (id) {
        id -> Int4,
        content_id -> Int4,
        direction -> Direction,
        name -> Text,
    }
}

diesel::table! {
    logs (id) {
        id -> Int4,
        log_id -> Text,
        name -> Nullable<Text>,
        validator -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    metadata (id) {
        id -> Int4,
        log_id -> Int4,
        record_id -> Int4,
        data -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::RecordStatus;

    records (id) {
        id -> Int4,
        log_id -> Int4,
        record_id -> Text,
        registry_log_index -> Nullable<Int8>,
        content -> Bytea,
        status -> RecordStatus,
        reason -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(contents -> records (record_id));
diesel::joinable!(interfaces -> contents (content_id));
diesel::joinable!(metadata -> logs (log_id));
diesel::joinable!(metadata -> records (record_id));
diesel::joinable!(records -> logs (log_id));

diesel::allow_tables_to_appear_in_same_query!(
    checkpoints,
    contents,
    interfaces,
    logs,
    metadata,
    records,
);
