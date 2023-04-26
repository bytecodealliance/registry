#!/usr/bin/env bash

function generate_locals {
  local SCRIPT_DIR PGPORT PGPASS PGPASS_FILE PSQL_FILE

  SCRIPT_DIR="$1"
  if [[ ! -d "$SCRIPT_DIR" ]]; then
    printf "generate_locals missing arguments (script_dir)\n" 1>&2
    return 1
  fi

  PGPASS_FILE="$SCRIPT_DIR/pgpass.local.conf"
  PSQL_FILE="$SCRIPT_DIR/psql.local.sh"

  PGPORT=$(docker port bytecodealliance-registry-db-1 5432 | sed -e 's/.*://g')
  PGPASS="localhost"
  PGPASS="$PGPASS:$PGPORT"
  PGPASS="$PGPASS:warg_registry:postgres"
  PGPASS="$PGPASS:welcome123"
  printf "%s\n" "$PGPASS" >"$PGPASS_FILE"
  chmod 600 "$PGPASS_FILE"

  cat >"$PSQL_FILE" <<EOF
  #!/usr/bin/env bash

  set -eou pipefail

  PGPASSFILE="$PGPASS_FILE" psql -h localhost -p $PGPORT -U postgres -n warg_registry "\$@"
EOF
  chmod +x "$PSQL_FILE"
}
