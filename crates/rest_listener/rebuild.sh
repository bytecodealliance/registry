#!/usr/bin/env bash
#
# Rebuilds the rust-server client and server stubs for warg REST protocol.

set -eou pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
if [[ ! -d "$SCRIPT_DIR" ]]; then
  printf "Unexpected error, calculated SCRIPT_DIR was not a directory: %s\n" "$SCRIPT_DIR" 1>&2
  exit 2
fi

REPO_DIR="$(realpath "$SCRIPT_DIR/../..")"
if [[ ! -d "$REPO_DIR" ]]; then
  printf "Unexpected error, calculated REPO_DIR was not a directory: %s\n" "$REPO_DIR" 1>&2
  exit 2
fi

if ! command -v docker >/dev/null; then
  cat >&2 <<EOF
Executable docker not in PATH. See the following to install:

https://docs.docker.com/get-docker/
EOF
  exit 2
fi

docker run --rm -v "${SCRIPT_DIR}:/out" \
  -v "${REPO_DIR}/openapi:/openapi" \
  openapitools/openapi-generator-cli generate \
  -i /openapi/warg/protocol/v1/service.swagger.json \
  -g rust-server \
  -o /out
