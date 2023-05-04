#!/usr/bin/env bash
#
# Starts the local infra.

set -eou pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"

if [[ ! -d "$SCRIPT_DIR" ]]; then
  printf "Unexpected error, calculated SCRIPT_DIR was not a directory: %s\n" "$SCRIPT_DIR" 1>&2
  exit 2
fi

# SEE: https://github.com/docker/buildx/issues/197
REPO_DIR="$(realpath "$SCRIPT_DIR/../..")"

if [[ ! -d "$REPO_DIR" ]]; then
  printf "Unexpected error, calculated REPO_DIR was not a directory: %s\n" "$REPO_DIR" 1>&2
  exit 2
fi

# shellcheck disable=SC1091
. "$SCRIPT_DIR/locals.inc.sh"

# shellcheck disable=SC1091
. "$SCRIPT_DIR/secrets.inc.sh"

generate_secrets

docker-compose --env-file "$SCRIPT_DIR/.env" \
  -f "$REPO_DIR/docker-compose.yaml" \
  -f "$REPO_DIR/docker-compose.postgres.yaml" \
  up -d

generate_locals "$SCRIPT_DIR"
