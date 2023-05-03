#!/usr/bin/env bash
#
# Stops the local infra and removes any associated data and container resources.

set -eou pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
if [[ ! -d "$SCRIPT_DIR" ]]; then
  printf "Unexpected error, calculated SCRIPT_DIR was not a directory: %s\n" "$SCRIPT_DIR" 1>&2
  exit 2
fi

REPO_DIR="$(realpath "$SCRIPT_DIR/..")"
if [[ ! -d "$REPO_DIR" ]]; then
  printf "Unexpected error, calculated REPO_DIR was not a directory: %s\n" "$REPO_DIR" 1>&2
  exit 2
fi

if ! command -v protoc-gen-openapiv2 >/dev/null; then
  cat >&2 <<EOF
Executable protoc-gen-openapiv2 not in PATH. Use the following to install:

go install \
    github.com/grpc-ecosystem/grpc-gateway/v2/protoc-gen-openapiv2@latest

EOF
  exit 2
fi

PROTO_DIR="$REPO_DIR/proto"
SERVICE_PROTO_FILES=("warg/protocol/v1/service.proto")


protoc -I "$PROTO_DIR" --openapiv2_out "$SCRIPT_DIR" \
    --openapiv2_opt logtostderr=true \
    "${SERVICE_PROTO_FILES[@]}"
