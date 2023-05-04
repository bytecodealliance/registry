#!/usr/bin/env bash
#
# Builds the secrets needed for the local infra if not already present.

function generate_password {
  # macOS (because its tr cannot handle urandom input), or Linux if installed
  if command -v uuidgen >/dev/null; then
    uuidgen
  else
    tr </dev/urandom -dc '[:alnum:]' | head -c 16
  fi
}

function generate_secrets {
  local SECRETS_DIR POSTGRES_CREDS_DIR

  if [[ ! -d "${REPO_DIR:-}" ]]; then
    printf "The environment variable REPO_DIR was not set nor a directory.\n" 1>&2
  fi

  SECRETS_DIR="$REPO_DIR/infra/local/.secrets"
  POSTGRES_CREDS_DIR="$SECRETS_DIR/data-store/postgres"

  mkdir -p "$POSTGRES_CREDS_DIR"

  # NOTE: the password should not require escaping/encoding for a URL
  if [[ ! -f "$POSTGRES_CREDS_DIR/password" ]]; then
    echo -n "$(generate_password)" >"$POSTGRES_CREDS_DIR/password"
  fi

  if [[ ! -f "$POSTGRES_CREDS_DIR/database_url" ]]; then
    echo -n "postgres://postgres:$(<"$POSTGRES_CREDS_DIR/password")@db:5432/warg_registry" >"$POSTGRES_CREDS_DIR/database_url"
  fi
  if [[ ! -f "$POSTGRES_CREDS_DIR/database_url_env" ]]; then
    echo "DATABASE_URL=$(<"$POSTGRES_CREDS_DIR/database_url")" >"$POSTGRES_CREDS_DIR/database_url_env"
  fi

  # TODO: generate operator-key dynamically
  echo -n "ecdsa-p256:I+UlDo0HxyBBFeelhPPWmD+LnklOpqZDkrFP5VduASk=" >"$SECRETS_DIR/operator_key"
}
