-- Your SQL goes here

CREATE TYPE DIRECTION AS ENUM('import', 'export');
CREATE TABLE interfaces (
  id SERIAL PRIMARY KEY,
  content_id INTEGER NOT NULL REFERENCES contents(id),
  direction DIRECTION NOT NULL,
  name TEXT NOT NULL
);