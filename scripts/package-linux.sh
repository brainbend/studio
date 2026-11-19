#!/bin/sh
# Builds the Linux downloads in release-files/ from an already built release binary (Linux only):
#   BrainBend-Studio-linux[-aarch64].tar.gz   the in-app updater downloads this one
#   brainbend-studio_<amd64|arm64>.deb
#   brainbend-studio.<x86_64|aarch64>.rpm
#   BrainBend-Studio-<x86_64|aarch64>.AppImage
# None of the names has a version in it, so a link to the latest release never changes; the packages and the app say which version they are.
# The in-app updater finds its file by the ending of the name, so keep these endings.
# Usage: scripts/package-linux.sh <tag>      (the tag is the version inside the packages; arch is this machine's; the binary is target/release/brainbend-studio)
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
tar -czf "$OUT/BrainBend-Studio-$TARSUFFIX.tar.gz" -C target/release brainbend-studio -C ../.. $LICENSES

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
dpkg-deb --root-owner-group --build "$DEBROOT" "$OUT/brainbend-studio_$DEB.deb"

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
# No version in the file name (the package itself has it), so a link to the latest release never changes.
for f in "$TOP"/RPMS/*/*.rpm; do cp "$f" "$OUT/brainbend-studio.$(basename "$(dirname "$f")").rpm"; done

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
ARCH=$ARCH "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT/BrainBend-Studio-$ARCH.AppImage"
ls -la "$OUT"
