#!/usr/bin/env bash
# Writes THIRD-PARTY-LICENSES.md: the license text and copyright notice of every dependency that ships in
# a download. Fails if a dependency uses a license that is not listed in about.toml.
# Needs: cargo install cargo-about --locked --features cli
set -euo pipefail
cd "$(dirname "$0")/.."
cargo about generate -m apps/brainbend-studio/Cargo.toml about.hbs -o THIRD-PARTY-LICENSES.md
echo "Wrote THIRD-PARTY-LICENSES.md"
