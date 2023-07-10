#!/bin/sh

echo -n "${DATABASE_URL}" > database_url
export WARG_DATABASE_URL_FILE=database_url
export WARG_DATA_STORE=postgres
export WARG_CONTENT_DIR=/tmp

/usr/local/bin/warg-server --database-run-migrations "$@"
