-- Your SQL goes here
CREATE TABLE metadata (
  id SERIAL PRIMARY KEY,
  log_id INTEGER NOT NULL REFERENCES logs(id),
  record_id INTEGER NOT NULL REFERENCES records(id),
  data JSONB
);