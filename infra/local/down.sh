#!/usr/bin/env bash
#
# Stops the local infra and removes any associated data and container resources.

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

docker-compose --env-file "$SCRIPT_DIR/.env" \
  -f "$REPO_DIR/docker-compose.yaml" \
  -f "$REPO_DIR/docker-compose.postgres.yaml" \
  down --remove-orphans --volumes

find "$SCRIPT_DIR" -type f -name '*.local.*' -exec rm -f '{}' \;
rm -rf "$SCRIPT_DIR/.secrets"
