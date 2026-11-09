#!/bin/sh
# Builds the Linux downloads in release-files/ from an already built release binary (Linux only):
#   BrainBend-Studio-<tag>-linux[-aarch64].tar.gz   the in-app updater downloads this one
#   brainbend-studio_<version>_<amd64|arm64>.deb
#   brainbend-studio-<version>.<x86_64|aarch64>.rpm
#   BrainBend-Studio-<tag>-<x86_64|aarch64>.AppImage
# Usage: scripts/package-linux.sh <tag>      (arch is this machine's; the binary is target/release/brainbend-studio)
set -eu
cd "$(dirname "$0")/.."
TAG=$1
VERSION=${TAG#v}
BIN=target/release/brainbend-studio
case "$(uname -m)" in
  x86_64)  ARCH=x86_64;  DEB=amd64; TARSUFFIX=linux;;
  aarch64) ARCH=aarch64; DEB=arm64; TARSUFFIX=linux-aarch64;;
  *) echo "Unsupported CPU $(uname -m)"; exit 1;;
esac
OUT=release-files; mkdir -p "$OUT"
LICENSES="LICENSE-MIT LICENSE-APACHE THIRD-PARTY-LICENSES.md FONT-LICENSES.md"
python3 scripts/make-icon.py --extras target/icons
ICON=target/icons/brainbend-studio.png

# 1. tar.gz
tar -czf "$OUT/BrainBend-Studio-$TAG-$TARSUFFIX.tar.gz" -C target/release brainbend-studio -C ../.. $LICENSES

# The files every package installs, laid out as under /.
ROOT=$(mktemp -d)
install -Dm755 "$BIN" "$ROOT/usr/bin/brainbend-studio"
install -Dm644 packaging/linux/brainbend-studio.desktop "$ROOT/usr/share/applications/brainbend-studio.desktop"
install -Dm644 packaging/linux/brainbend-studio.xml "$ROOT/usr/share/mime/packages/brainbend-studio.xml"
install -Dm644 "$ICON" "$ROOT/usr/share/icons/hicolor/512x512/apps/brainbend-studio.png"
for f in $LICENSES; do install -Dm644 "$f" "$ROOT/usr/share/doc/brainbend-studio/$f"; done

# 2. deb
DEBROOT=$(mktemp -d)
cp -a "$ROOT/." "$DEBROOT/"
mkdir -p "$DEBROOT/DEBIAN"
SIZE=$(du -sk "$ROOT" | cut -f1)
cat > "$DEBROOT/DEBIAN/control" <<CTL
Package: brainbend-studio
Version: $VERSION
Architecture: $DEB
Maintainer: BrainBend <support@brainbend.studio>
Installed-Size: $SIZE
Section: education
Priority: optional
Homepage: https://github.com/brainbend/studio
Description: Author interactive lessons
 BrainBend Studio is a desktop app for building interactive lessons
 and publishing them for the web and learning management systems.
CTL
dpkg-deb --root-owner-group --build "$DEBROOT" "$OUT/brainbend-studio_${VERSION}_$DEB.deb"

# 3. rpm (the version may not hold a hyphen)
RPMVER=$(printf '%s' "$VERSION" | tr '-' '~')
TOP=$(mktemp -d)
cat > "$TOP/studio.spec" <<SPEC
Name: brainbend-studio
Version: $RPMVER
Release: 1
Summary: Author interactive lessons
License: MIT OR Apache-2.0
URL: https://github.com/brainbend/studio
AutoReqProv: no
%description
BrainBend Studio is a desktop app for building interactive lessons and publishing them for the web and learning management systems.
%install
mkdir -p %{buildroot}
cp -a $ROOT/. %{buildroot}/
%files
/usr/bin/brainbend-studio
/usr/share/applications/brainbend-studio.desktop
/usr/share/mime/packages/brainbend-studio.xml
/usr/share/icons/hicolor/512x512/apps/brainbend-studio.png
/usr/share/doc/brainbend-studio
SPEC
rpmbuild -bb --define "_topdir $TOP" --define "_build_id_links none" --define "debug_package %{nil}" "$TOP/studio.spec"
cp "$TOP"/RPMS/*/*.rpm "$OUT/"

# 4. AppImage
APPDIR=$(mktemp -d)/BrainBend.AppDir
mkdir -p "$APPDIR"
cp -a "$ROOT/usr" "$APPDIR/"
cp packaging/linux/brainbend-studio.desktop "$APPDIR/"
cp "$ICON" "$APPDIR/brainbend-studio.png"
ln -s usr/bin/brainbend-studio "$APPDIR/AppRun"
TOOL=target/appimagetool
if [ ! -x "$TOOL" ]; then
  curl -fsSL -o "$TOOL" "https://github.com/AppImage/appimagetool/releases/download/1.9.0/appimagetool-$ARCH.AppImage"
  chmod +x "$TOOL"
fi
ARCH=$ARCH "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT/BrainBend-Studio-$TAG-$ARCH.AppImage"
ls -la "$OUT"
