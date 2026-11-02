#!/bin/sh
# Builds a release binary and wraps it in dist/BrainBend Studio.app (macOS only).
set -eu
cd "$(dirname "$0")/.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' apps/brainbend-studio/Cargo.toml | head -1)
cargo build --release -p brainbend-studio
APP="dist/BrainBend Studio.app"
rm -rf "${APP:?}"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/brainbend-studio "$APP/Contents/MacOS/BrainBend Studio"
ICONSET="$(mktemp -d)/BrainBend Studio.iconset"
python3 scripts/make-icon.py "$ICONSET"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/BrainBend Studio.icns"
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
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
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
echo "Built $APP"
