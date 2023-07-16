CREATE TABLE dependencies (
  id SERIAL PRIMARY KEY,
  log_id TEXT NOT NULL,
  record_id TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  version TEXT,
  location TEXT,
  integrity TEXT
);