#!/bin/bash
set -euo pipefail

function kill_postgres() {
    echo killing postgres container
    docker kill postgres-test &> /dev/null || true
    docker rm postgres-test &> /dev/null || true
    rm -rf /tmp/postgres-test &> /dev/null || true
}

trap 'kill_postgres' EXIT

kill_postgres

echo starting postgres container
docker run -d --name postgres-test -e POSTGRES_PASSWORD=password -v /tmp/postgres-test:/var/lib/postgresql/data -p 5433:5432 postgres

while ! docker exec postgres-test pg_isready; do
    echo waiting for postgres to accept connections
    sleep 1
done

echo setting up database
diesel database setup --database-url postgres://postgres:password@localhost:5433/test-registry --migration-dir crates/server/src/datastore/postgres/migrations

echo running tests
DATABASE_URL=postgres://postgres:password@localhost:5433/test-registry cargo test --features postgres $@
