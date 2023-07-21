-- Stores every checkpoint performed by the registry
CREATE TABLE checkpoints (
  id SERIAL PRIMARY KEY,
  checkpoint_id TEXT NOT NULL UNIQUE,
  log_root TEXT NOT NULL,
  log_length BIGINT NOT NULL,
  map_root TEXT NOT NULL,
  key_id TEXT NOT NULL,
  signature TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('checkpoints');

-- Unified table for both package and operator logs.
-- The `name` column is NULL for the operator log.
CREATE TABLE logs (
  id SERIAL PRIMARY KEY,
  log_id TEXT NOT NULL UNIQUE,
  name TEXT, -- implied UNIQUE constraint as log_id is derived from name
  validator JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('logs');

CREATE TYPE record_status AS ENUM ('pending', 'rejected', 'validated');

-- Unified table for both package and operator log records.
CREATE TABLE records (
  id SERIAL PRIMARY KEY,
  log_id INTEGER NOT NULL REFERENCES logs(id),
  record_id TEXT NOT NULL UNIQUE,
  registry_log_index BIGINT UNIQUE,
  content BYTEA NOT NULL,
  status record_status NOT NULL DEFAULT 'pending',
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('records');

-- Represents record contents.
-- Note that while digests may be repeated here (as these are per-record),
-- only one copy of the content matching the digest is ever stored.
CREATE TABLE contents (
  id SERIAL PRIMARY KEY,
  record_id INTEGER NOT NULL REFERENCES records(id),
  digest TEXT NOT NULL,
  missing BOOLEAN NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX contents_digest_record_id_idx ON contents (record_id, digest);

SELECT diesel_manage_updated_at('contents');
