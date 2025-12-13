#!/bin/sh

set -e

cargo build --release --target-dir=/tmp/codecrafters-build-shell-rust --manifest-path Cargo.toml
