#!/bin/sh

set -e

(
  # Ensure compile steps are run within the repository directory
  cd "$(dirname "$0")"
  cargo build --release --target-dir=/tmp/codecrafters-build-shell-rust --manifest-path Cargo.toml
)

exec /tmp/codecrafters-build-shell-rust/release/codecrafters-shell "$@"
