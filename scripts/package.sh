#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
PKG="crates/oryn-app/packaging"
DIST="$ROOT/dist"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
rm -rf "$DIST"
mkdir -p "$DIST"

if [ "${SKIP_BUILD:-}" != "1" ]; then
  cargo build --release --locked -p oryn-app
fi
BIN="$TARGET_DIR/release/oryn"

case "$(uname -s)" in
  Darwin)
    OS=macos
    ARCH="$(uname -m)"
    [ "$ARCH" = "arm64" ] && ARCH="aarch64"
    APP="$DIST/Oryn.app"
    DMG="$DIST/Oryn_${VERSION}_${ARCH}.dmg"
    if otool -L "$BIN" | grep -E '^\s+/(opt/homebrew|usr/local)/'; then
      echo "oryn links to the libraries above, which users do not have" >&2
      exit 1
    fi
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp "$BIN" "$APP/Contents/MacOS/oryn"
    cp "$PKG/icons/icon.icns" "$APP/Contents/Resources/icon.icns"
    sed "s/@VERSION@/$VERSION/g" "$PKG/macos/Info.plist" > "$APP/Contents/Info.plist"
    notarize() {
      xcrun notarytool submit "$1" --wait \
        --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER"
    }
    NOTARIZE=
    if [ -n "${MACOS_SIGNING_IDENTITY:-}" ]; then
      codesign --force --options runtime --timestamp -s "$MACOS_SIGNING_IDENTITY" "$APP"
      [ -n "${APPLE_API_KEY_PATH:-}" ] && NOTARIZE=1
    else
      codesign --force -s - "$APP"
    fi
    if [ -n "$NOTARIZE" ]; then
      (cd "$DIST" && ditto -c -k --keepParent Oryn.app notarize.zip)
      notarize "$DIST/notarize.zip"
      rm "$DIST/notarize.zip"
      xcrun stapler staple "$APP"
    fi
    STAGE="$(mktemp -d)"
    cp -R "$APP" "$STAGE/"
    ln -s /Applications "$STAGE/Applications"
    hdiutil create -volname "Oryn" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
    if [ -n "$NOTARIZE" ]; then
      codesign --timestamp -s "$MACOS_SIGNING_IDENTITY" "$DMG"
      notarize "$DMG"
      xcrun stapler staple "$DMG"
    fi
    (cd "$DIST" && ditto -c -k --keepParent Oryn.app "Oryn_${VERSION}_${ARCH}.app.zip")
    rm -rf "$STAGE" "$APP"
    ;;
  Linux)
    OS=linux
    ARCH="$(uname -m)"
    DEB_ARCH="$(dpkg --print-architecture 2>/dev/null || echo amd64)"
    NAME="Oryn_${VERSION}_linux_${ARCH}"
    STAGE="$DIST/$NAME"
    mkdir -p "$STAGE"
    cp "$BIN" "$STAGE/oryn"
    cp "$PKG/linux/com.kaventro.Oryn.desktop" "$STAGE/"
    cp "$PKG/icons/icon.png" "$STAGE/com.kaventro.Oryn.png"
    cp LICENSE "$STAGE/"
    tar -C "$DIST" -czf "$DIST/$NAME.tar.gz" "$NAME"
    rm -rf "$STAGE"

    DEB="$(mktemp -d)"
    chmod 755 "$DEB"
    install -Dm755 "$BIN" "$DEB/usr/bin/oryn"
    install -Dm644 "$PKG/linux/com.kaventro.Oryn.desktop" "$DEB/usr/share/applications/com.kaventro.Oryn.desktop"
    install -Dm644 "$PKG/icons/32x32.png" "$DEB/usr/share/icons/hicolor/32x32/apps/com.kaventro.Oryn.png"
    install -Dm644 "$PKG/icons/128x128.png" "$DEB/usr/share/icons/hicolor/128x128/apps/com.kaventro.Oryn.png"
    install -Dm644 "$PKG/icons/128x128@2x.png" "$DEB/usr/share/icons/hicolor/256x256/apps/com.kaventro.Oryn.png"
    install -Dm644 LICENSE "$DEB/usr/share/doc/oryn/copyright"
    mkdir -p "$DEB/DEBIAN"
    cat > "$DEB/DEBIAN/control" <<CONTROL
Package: oryn
Version: $VERSION
Section: utils
Priority: optional
Architecture: $DEB_ARCH
Maintainer: Kaventro <noreply@github.com>
Depends: libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libvulkan1, libx11-xcb1, libxcb1, libfontconfig1, libfreetype6, libzstd1
Homepage: https://github.com/kaventro/Oryn
Description: Fast dual-pane file manager
 Oryn is a native file manager with dual panes, Quick View,
 a built-in terminal and power tools.
CONTROL
    dpkg-deb --build --root-owner-group "$DEB" "$DIST/oryn_${VERSION}_${DEB_ARCH}.deb"
    rm -rf "$DEB"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    OS=windows
    EXE="$TARGET_DIR/release/oryn.exe"
    cp "$EXE" "$DIST/Oryn_${VERSION}_x64_portable.exe"
    NSIS="$(command -v makensis || true)"
    if [ -z "$NSIS" ] && [ -x "/c/Program Files (x86)/NSIS/makensis.exe" ]; then
      NSIS="/c/Program Files (x86)/NSIS/makensis.exe"
    fi
    if [ -n "$NSIS" ]; then
      W() { cygpath -w "$1"; }
      "$NSIS" -V2 \
        "-DVERSION=$VERSION" \
        "-DEXE=$(W "$EXE")" \
        "-DICON=$(W "$ROOT/$PKG/icons/icon.ico")" \
        "-DOUTFILE=$(W "$DIST/Oryn_${VERSION}_x64-setup.exe")" \
        "$(W "$ROOT/$PKG/windows/installer.nsi")"
    else
      echo "makensis not found, skipping the installer" >&2
    fi
    ;;
  *)
    echo "unsupported platform $(uname -s)" >&2
    exit 1
    ;;
esac

SUMS="$DIST/SHA256SUMS-$OS.txt"
(cd "$DIST" && for f in Oryn* oryn*; do [ -f "$f" ] || continue; sha256sum "$f" 2>/dev/null || shasum -a 256 "$f"; done) > "$SUMS.tmp"
mv "$SUMS.tmp" "$SUMS"
ls -la "$DIST"
