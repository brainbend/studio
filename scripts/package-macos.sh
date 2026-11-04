#!/bin/sh
# Builds dist/brainbend-studio-<version>-macos-<universal|arch>.dmg (macOS only).
#   scripts/package-macos.sh universal   Apple silicon + Intel in one app (needs both rust targets installed)
#   scripts/package-macos.sh             this Mac's architecture only
set -eu
cd "$(dirname "$0")/.."
VERSION=$(scripts/version.sh)
if [ "${1:-native}" = universal ]; then
  cargo build --release -p brainbend-studio --target aarch64-apple-darwin
  cargo build --release -p brainbend-studio --target x86_64-apple-darwin
  mkdir -p target/universal
  lipo -create -output target/universal/brainbend-studio \
    target/aarch64-apple-darwin/release/brainbend-studio target/x86_64-apple-darwin/release/brainbend-studio
  BINARY=target/universal/brainbend-studio scripts/bundle-macos.sh
  SUFFIX=universal
else
  scripts/bundle-macos.sh
  SUFFIX=$(uname -m)
fi
STAGE=$(mktemp -d)
cp -R "dist/BrainBend Studio.app" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
DMG="dist/brainbend-studio-$VERSION-macos-$SUFFIX.dmg"
rm -f "$DMG"
# Give the disk image room explicitly: on some macOS versions hdiutil's own size estimate comes out too small and
# fails with "No space left on device" even when the disk is nearly empty.
SIZE_KB=$(du -sk "$STAGE" | cut -f1)
hdiutil create -volname "BrainBend Studio" -srcfolder "$STAGE" -ov -fs HFS+ -size $((SIZE_KB * 2 + 51200))k -format UDZO "$DMG"
echo "Built $DMG"
