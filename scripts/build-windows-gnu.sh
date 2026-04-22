#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-gnu}"
PROFILE="${PROFILE:-release}"
APP_BIN="${APP_BIN:-termvoid}"
PKG_CONFIG_WRAPPER="${PKG_CONFIG_WRAPPER:-x86_64-w64-mingw32-pkg-config}"
MINGW_LINKER="${MINGW_LINKER:-x86_64-w64-mingw32-gcc}"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
DIST_ROOT="${DIST_ROOT:-${REPO_DIR}/dist/windows-gnu}"
BUNDLE_DIR="${BUNDLE_DIR:-${DIST_ROOT}/${APP_BIN}-${TARGET}-${PROFILE}}"

require_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        printf 'Missing required command: %s\n' "$1" >&2
        exit 1
    fi
}

profile_dir() {
    if [[ "$PROFILE" == "release" ]]; then
        printf 'release\n'
    else
        printf '%s\n' "$PROFILE"
    fi
}

guess_mingw_prefix() {
    local candidate

    for candidate in \
        "${MINGW_PREFIX:-}" \
        "/usr/x86_64-w64-mingw32/sys-root/mingw" \
        "/usr/x86_64-w64-mingw32" \
        "/usr/x86_64-w64-mingw32/sys-root/ucrt64"
    do
        [[ -n "$candidate" ]] || continue
        [[ -d "$candidate/bin" ]] || continue
        [[ -d "$candidate/share" ]] || continue
        [[ -d "$candidate/lib" ]] || continue
        printf '%s\n' "$candidate"
        return 0
    done

    return 1
}

require_cmd cargo
require_cmd rustup
require_cmd "$PKG_CONFIG_WRAPPER"
require_cmd "$MINGW_LINKER"

if ! rustup target list --installed | grep -qx "$TARGET"; then
    printf 'Rust target %s is not installed.\nRun: rustup target add %s\n' "$TARGET" "$TARGET" >&2
    exit 1
fi

MINGW_PREFIX_RESOLVED="${MINGW_PREFIX:-}"
if [[ -z "$MINGW_PREFIX_RESOLVED" ]]; then
    if ! MINGW_PREFIX_RESOLVED="$(guess_mingw_prefix)"; then
        cat >&2 <<'EOF'
Unable to locate a MinGW GTK runtime prefix.
Set MINGW_PREFIX to the directory that contains bin/, share/ and lib/.

Example:
  MINGW_PREFIX=/usr/x86_64-w64-mingw32/sys-root/mingw ./scripts/build-windows-gnu.sh
EOF
        exit 1
    fi
fi

EXE_PATH="${REPO_DIR}/target/${TARGET}/$(profile_dir)/${APP_BIN}.exe"

if [[ -e "$BUNDLE_DIR" ]]; then
    printf 'Bundle directory already exists: %s\nChoose another BUNDLE_DIR or remove it first.\n' "$BUNDLE_DIR" >&2
    exit 1
fi

BUILD_ARGS=(build --target "$TARGET")
if [[ "$PROFILE" == "release" ]]; then
    BUILD_ARGS+=(--release)
else
    BUILD_ARGS+=(--profile "$PROFILE")
fi

(
    cd "$REPO_DIR"
    PKG_CONFIG="$PKG_CONFIG_WRAPPER" \
    PKG_CONFIG_ALLOW_CROSS=1 \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="$MINGW_LINKER" \
    cargo "${BUILD_ARGS[@]}"
)

if [[ ! -f "$EXE_PATH" ]]; then
    printf 'Build finished but executable was not found: %s\n' "$EXE_PATH" >&2
    exit 1
fi

install -d "$BUNDLE_DIR"
cp "$EXE_PATH" "$BUNDLE_DIR/"
cp -a "$MINGW_PREFIX_RESOLVED/bin" "$BUNDLE_DIR/"
cp -a "$MINGW_PREFIX_RESOLVED/share" "$BUNDLE_DIR/"
cp -a "$MINGW_PREFIX_RESOLVED/lib" "$BUNDLE_DIR/"

install -d "$BUNDLE_DIR/share/icons"
cp -a "$REPO_DIR/assets/icons/hicolor" "$BUNDLE_DIR/share/icons/"

cat <<EOF
Windows bundle created at:
  $BUNDLE_DIR

The portable layout now contains:
  termvoid.exe
  bin/
  share/
  lib/

You can zip that directory and run termvoid.exe on Windows.
EOF
