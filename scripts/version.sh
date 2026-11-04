#!/bin/sh
# Prints the app version (from the app's Cargo.toml). VERSION_OVERRIDE wins, for test builds like 0.2.0-rc.1.
set -eu
if [ -n "${VERSION_OVERRIDE:-}" ]; then echo "$VERSION_OVERRIDE"; exit 0; fi
cd "$(dirname "$0")/.."
sed -n 's/^version = "\(.*\)"/\1/p' apps/brainbend-studio/Cargo.toml | head -1
