#!/bin/bash
set -euo pipefail

function stop_postgres() {
    echo stopping postgres container
    docker stop postgres-test &> /dev/null || true
    docker rm postgres-test &> /dev/null || true
}

trap 'stop_postgres' EXIT

stop_postgres

echo starting postgres container
docker run -d --name postgres-test -e POSTGRES_PASSWORD=password -p 5433:5432 postgres

while ! docker exec postgres-test pg_isready; do
    echo waiting for postgres to accept connections
    sleep 1
done

echo setting up database
diesel database setup --database-url postgres://postgres:password@localhost:5433/test-registry --migration-dir crates/server/src/datastore/postgres/migrations

echo running tests
WARG_DATABASE_URL=postgres://postgres:password@localhost:5433/test-registry cargo test --features postgres $@
