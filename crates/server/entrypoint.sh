#!/bin/bash
set -e

exec warg-server --content-dir "$CONTENT_DIR"
