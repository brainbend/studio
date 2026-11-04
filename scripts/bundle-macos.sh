#!/bin/sh
# Wraps the app in dist/BrainBend Studio.app (macOS only).
#   scripts/bundle-macos.sh                 builds the release binary for this Mac, then wraps it
#   BINARY=path scripts/bundle-macos.sh     wraps an existing binary (for example a universal one)
set -eu
cd "$(dirname "$0")/.."
VERSION=$(scripts/version.sh)
PLIST_VERSION=${VERSION%%-*}   # the bundle version allows digits and dots only
if [ -z "${BINARY:-}" ]; then
  cargo build --release -p brainbend-studio
  BINARY=target/release/brainbend-studio
fi
APP="dist/BrainBend Studio.app"
rm -rf "${APP:?}"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/BrainBend Studio"
ICONSET="$(mktemp -d)/BrainBend Studio.iconset"
python3 scripts/make-icon.py "$ICONSET"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/BrainBend Studio.icns"
# The licenses travel with the app: ours, and the notices the open-source parts require.
cp LICENSE-MIT LICENSE-APACHE THIRD-PARTY-LICENSES.md "$APP/Contents/Resources/"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>BrainBend Studio</string>
  <key>CFBundleDisplayName</key><string>BrainBend Studio</string>
  <key>CFBundleIdentifier</key><string>com.brainbend.studio</string>
  <key>CFBundleExecutable</key><string>BrainBend Studio</string>
  <key>CFBundleIconFile</key><string>BrainBend Studio</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${PLIST_VERSION}</string>
  <key>CFBundleVersion</key><string>${PLIST_VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleDocumentTypes</key><array>
    <dict><key>CFBundleTypeName</key><string>BrainBend Studio project</string>
      <key>CFBundleTypeExtensions</key><array><string>brainbend</string></array>
      <key>CFBundleTypeRole</key><string>Editor</string></dict>
    <dict><key>CFBundleTypeName</key><string>Course file</string>
      <key>CFBundleTypeExtensions</key><array><string>story</string></array>
      <key>CFBundleTypeRole</key><string>Viewer</string></dict>
  </array>
</dict></plist>
PLIST
# Developer ID signing and notarization come later; an ad-hoc signature keeps the bundle valid on Apple silicon.
codesign --force --deep --sign - "$APP"
echo "Built $APP"
