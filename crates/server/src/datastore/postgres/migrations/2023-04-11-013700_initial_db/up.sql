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
-- The nullable columns are null for operator logs.
-- TODO: add full text search indexes for description and keywords.
CREATE TABLE logs (
  id SERIAL PRIMARY KEY,
  log_id TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL, -- implied UNIQUE constraint as log_id is derived from name
  validator JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('logs');

CREATE TYPE record_status AS ENUM ('pending', 'rejected', 'accepted');

-- Unified table for both package and operator log records.
CREATE TABLE records (
  id SERIAL PRIMARY KEY,
  log_id INTEGER NOT NULL REFERENCES logs(id),
  record_id TEXT NOT NULL UNIQUE,
  checkpoint_id INTEGER REFERENCES checkpoints(id),
  content BYTEA NOT NULL,
  status record_status NOT NULL DEFAULT 'pending',
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('records');

CREATE TYPE source_kind AS ENUM ('http');

-- Represents content sources associated with a record.
-- Currently it is expected there is only a "http" kind of source.
CREATE TABLE sources (
  id SERIAL PRIMARY KEY,
  record_id INTEGER NOT NULL REFERENCES records(id),
  digest TEXT NOT NULL,
  kind source_kind NOT NULL,
  url TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('sources');
