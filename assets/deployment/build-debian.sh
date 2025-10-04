#!/bin/sh
set -ex

LTO="lto"
PACKAGE_DIR="target/packaged"
DEB_DIR="target/debian"
DEB_VER="1.0.0-1"
X86_MUSL="x86_64-unknown-linux-musl"
X86_GNU="x86_64-unknown-linux-gnu"
AARCH64_MUSL="aarch64-unknown-linux-musl"
AARCH64_GNU="aarch64-unknown-linux-gnu"
X86_WINDOWS="x86_64-pc-windows-gnu"
AARCH64_MAC="aarch64-apple-darwin"

function package() {
  BIN_NAME="$1"
  BUILD_TARGET="$2"
  OUTPUT_DIR="target/$BUILD_TARGET/$LTO"
  DEB_ARCH="$3"
  DEST_NAME="$4"
  EXT="$5"
  cross b -p "$BIN_NAME" --target "$BUILD_TARGET" --profile "$LTO"
  cp "$OUTPUT_DIR/$BIN_NAME""$EXT" "$PACKAGE_DIR/$DEST_NAME""$EXT"
  gpg --yes --output "$PACKAGE_DIR/$DEST_NAME.sig" --detach-sig "$PACKAGE_DIR/$DEST_NAME"
  # Some shared artifacts end up here, getting reused and causing build failures
  rm -r "target/$LTO"
}

function package_deb() {
   package "$1" "$2" "$3" "$4" "$5"
   DEB_ARCH="$3"
   DEB_FILE="$DEB_DIR/$BIN_NAME"_"$DEB_VER"_"$DEB_ARCH.deb"
   cargo deb -p "$BIN_NAME" --target "$BUILD_TARGET" --profile "$LTO" --no-build --no-strip
   gpg --yes --output "$DEB_FILE.sig" --detach-sig "$DEB_FILE"
   mv "$DEB_FILE.sig" "$PACKAGE_DIR"
   mv "$DEB_FILE" "$PACKAGE_DIR"
}

mkdir -p "$PACKAGE_DIR"
package_deb "p2proxyd" "$X86_MUSL" "amd64" "p2proxyd-x86_64-linux" ""
package_deb "p2proxyd" "$AARCH64_MUSL" "arm64" "p2proxyd-aarch64-linux" ""
package_deb "p2proxy-cli" "$X86_MUSL" "amd64" "p2proxy-cli-x86_64-linux" ""
package_deb "p2proxy-cli" "$AARCH64_MUSL" "arm64" "p2proxy-cli-aarch64-linux" ""

# Inject a prepared image to build with (has deps and an old debian version for compatibility)
export CROSS_CONFIG=./p2proxy-desktop/cross.toml
package_deb "p2proxy-desktop" "$X86_GNU" "amd64" "p2proxy-desktop-x86_64-linux-gnu" ""

package "p2proxy-cli" "$X86_WINDOWS" "amd64" "p2proxy-cli" ".exe"
package "p2proxy-desktop" "$X86_WINDOWS" "amd64" "p2proxy-desktop" ".exe"
