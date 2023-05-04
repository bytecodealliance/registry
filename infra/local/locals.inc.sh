#!/usr/bin/env bash
#
# Combines locally generated secrets with runtime information for local
# connection utilities.

function generate_locals {
  if [[ ! -d "${REPO_DIR:-}" ]]; then
    printf "The environment variable REPO_DIR was not set nor a directory.\n" 1>&2
  fi

  local LOCAL_INFRA_DIR SECRETS_DIR PGPORT PGPASS PGPASS_FILE PSQL_FILE

  LOCAL_INFRA_DIR="$REPO_DIR/infra/local"
  SECRETS_DIR="$LOCAL_INFRA_DIR/.secrets"

  PGPASS_FILE="$SECRETS_DIR/pgpass.local.conf"
  PSQL_FILE="$SCRIPT_DIR/psql.local.sh"

  # Extract the local randomly bound port number used for exposing postgres.
  PGPORT=$(docker port bytecodealliance-registry-db-1 5432 | sed -e 's/.*://g')

  PGPASS="localhost"
  PGPASS="$PGPASS:$PGPORT"
  PGPASS="$PGPASS:warg_registry"
  PGPASS="$PGPASS:postgres"
  PGPASS="$PGPASS:$(<"$SECRETS_DIR/data-store/postgres/password")"
  printf "%s\n" "$PGPASS" >"$PGPASS_FILE"
  chmod 600 "$PGPASS_FILE"

  cat >"$PSQL_FILE" <<EOF
  #!/usr/bin/env bash

  set -eou pipefail

  PGPASSFILE="$PGPASS_FILE" psql -h "localhost" -p "$PGPORT" -U "postgres" -n "warg_registry" "\$@"
EOF
  chmod +x "$PSQL_FILE"
}
