#!/bin/sh
# Installs helix-trainer from a pre-built GitHub release binary.
#
# Usage:
#   LATEST_TAG=$(curl -fsSLI -o /dev/null -w '%{url_effective}' https://github.com/bug-ops/helix-trainer/releases/latest | sed 's#.*/tag/##')
#   (T="$(mktemp)" && trap 'rm -f "$T"' EXIT && curl -fsSL "https://raw.githubusercontent.com/bug-ops/helix-trainer/${LATEST_TAG}/scripts/install.sh" -o "$T" && sh "$T")
#   ./scripts/install.sh [-v VERSION] [-d DIR] [--static]
#
# Options:
#   -v, --version VERSION   Release to install, e.g. "0.5.12" (default: latest)
#   -d, --dir DIR           Install directory (default: $HOME/.local/bin)
#       --static            Prefer the statically-linked musl build on Linux
#                            (no audio feature)
#   -h, --help              Show this help message
#
# Windows users: run scripts/install.ps1 instead.
#
# Archive extraction prefers exarch (https://github.com/bug-ops/exarch), a
# memory-safe extractor with path-traversal/zip-bomb/symlink protections,
# bootstrapping it into a scratch directory when not already on PATH. The
# exarch release version and its per-target SHA-256 checksums are pinned as
# constants below (EXARCH_VERSION / EXARCH_SHA256_*) instead of resolved
# from exarch's "latest" release at install time: trust is anchored in this
# repo (itself fetched at a pinned release tag above) rather than in a live
# call to a third-party release feed, whose checksum would otherwise come
# from the same origin as the binary it verifies. Falls back to the system
# tar if exarch is unavailable or unsupported on this host.

set -eu

REPO="bug-ops/helix-trainer"
BINARY="helix-trainer"
EXARCH_REPO="bug-ops/exarch"
# Bump EXARCH_VERSION and every EXARCH_SHA256_* below together, reading the
# new values from https://github.com/bug-ops/exarch/releases/tag/vX.Y.Z's
# published *.sha256 assets - never independently.
EXARCH_VERSION="0.6.0"
EXARCH_SHA256_LINUX_X86_64="f381376b968b893a52a111591f87c8d22e60d10aa539d732d0faf75137b17f9c"
EXARCH_SHA256_LINUX_AARCH64="05cd89b86346fcaa54828cf90f3d728b98cebae380326dbf27545f95fd94883a"
EXARCH_SHA256_MACOS_X86_64="937ed9b37c8c4cf284b47f6ab4120fd676f0cf7db0d6f58e2b7fe2027665881e"
EXARCH_SHA256_MACOS_AARCH64="3048a71698e67198f801101c7f4112368dc9719854b7e47db661cfe80b114a4c"
VERSION="${HELIX_TRAINER_VERSION:-latest}"
INSTALL_DIR="${HELIX_TRAINER_INSTALL_DIR:-$HOME/.local/bin}"
STATIC=0

log() { printf '%s\n' "$*"; }
err() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
    case "$1" in
        -v | --version)
            VERSION="$2"
            shift 2
            ;;
        -d | --dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --static)
            STATIC=1
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            err "unknown option: $1 (see --help)"
            ;;
    esac
done

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

need_cmd tar
need_cmd mktemp

fetch() {
    # fetch URL OUT_FILE
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        err "curl or wget is required"
    fi
}

fetch_stdout() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$1"
    else
        err "curl or wget is required"
    fi
}

compute_sha256() {
    # compute_sha256 FILE — echoes the hex digest to stdout
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        echo "sha256sum or shasum is required to verify the download" >&2
        return 1
    fi
}

verify_checksum() {
    # verify_checksum DIR ARCHIVE — expects "ARCHIVE.sha256" (sha256sum
    # format: "<hex-digest>  <filename>") alongside ARCHIVE in DIR. Always
    # does an explicit digest string-compare instead of trusting a checksum
    # tool's own exit code: on some platforms (e.g. macOS's /sbin/sha256sum)
    # `sha256sum -c` exits 0 on a checksum file with no parseable lines at
    # all, which would otherwise let a truncated or empty download through
    # as "verified". A missing/empty checksum file, or one that doesn't
    # reference ARCHIVE by its exact filename, is rejected the same way as
    # a digest mismatch.
    (
        cd "$1"
        ARCHIVE="$2"
        CHECKSUM_FILE="${ARCHIVE}.sha256"
        [ -s "$CHECKSUM_FILE" ] || exit 1
        EXPECTED="$(awk -v f="$ARCHIVE" '{ name = $2; sub(/^\*/, "", name); if (name == f) { print $1; exit } }' "$CHECKSUM_FILE")"
        [ -n "$EXPECTED" ] || exit 1
        ACTUAL="$(compute_sha256 "$ARCHIVE")" || exit 1
        [ "$EXPECTED" = "$ACTUAL" ]
    )
}

EXARCH_BIN=""

resolve_exarch() {
    if command -v exarch >/dev/null 2>&1 && exarch --version >/dev/null 2>&1; then
        EXARCH_BIN="exarch"
        return 0
    fi
    # exarch's own release binaries only cover gnu/darwin targets (no musl, no
    # Windows); the OS-ARCH gate below is the host running this script, not
    # the release target picked by --static, so a musl host (e.g. Alpine)
    # still needs the --version smoke test below to reject a glibc binary
    # that can't execute.
    case "${OS}-${ARCH}" in
        linux-x86_64)
            EXARCH_TARGET="x86_64-unknown-linux-gnu"
            EXARCH_SHA256="$EXARCH_SHA256_LINUX_X86_64"
            ;;
        linux-aarch64)
            EXARCH_TARGET="aarch64-unknown-linux-gnu"
            EXARCH_SHA256="$EXARCH_SHA256_LINUX_AARCH64"
            ;;
        macos-x86_64)
            EXARCH_TARGET="x86_64-apple-darwin"
            EXARCH_SHA256="$EXARCH_SHA256_MACOS_X86_64"
            ;;
        macos-aarch64)
            EXARCH_TARGET="aarch64-apple-darwin"
            EXARCH_SHA256="$EXARCH_SHA256_MACOS_AARCH64"
            ;;
        *) return 1 ;;
    esac

    EXARCH_ARCHIVE="exarch-${EXARCH_VERSION}-${EXARCH_TARGET}.tar.gz"
    EXARCH_BASE_URL="https://github.com/${EXARCH_REPO}/releases/download/v${EXARCH_VERSION}"

    fetch "${EXARCH_BASE_URL}/${EXARCH_ARCHIVE}" "${TMP_DIR}/${EXARCH_ARCHIVE}" 2>/dev/null || return 1
    # The expected digest is the pinned constant above, not a checksum fetched
    # from exarch's own release - fetching it from the same release the
    # binary comes from would only guard against corruption, not a
    # compromised release (whoever can publish an exarch release controls
    # both artifacts).
    printf '%s  %s\n' "$EXARCH_SHA256" "$EXARCH_ARCHIVE" >"${TMP_DIR}/${EXARCH_ARCHIVE}.sha256"
    verify_checksum "$TMP_DIR" "$EXARCH_ARCHIVE" >/dev/null 2>&1 || return 1

    EXARCH_DIR="${TMP_DIR}/exarch-bin"
    mkdir -p "$EXARCH_DIR"
    tar -xzf "${TMP_DIR}/${EXARCH_ARCHIVE}" -C "$EXARCH_DIR" 2>/dev/null || return 1
    EXARCH_BIN="${EXARCH_DIR}/exarch-${EXARCH_VERSION}-${EXARCH_TARGET}/exarch"
    [ -x "$EXARCH_BIN" ] && "$EXARCH_BIN" --version >/dev/null 2>&1 || return 1
}

detect_os() {
    case "$(uname -s)" in
        Linux) echo linux ;;
        Darwin) echo macos ;;
        *) err "unsupported OS: $(uname -s) (use scripts/install.ps1 on Windows)" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64 | amd64) echo x86_64 ;;
        arm64 | aarch64) echo aarch64 ;;
        *) err "unsupported architecture: $(uname -m)" ;;
    esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

case "$OS" in
    linux)
        if [ "$STATIC" -eq 1 ]; then
            TARGET="${ARCH}-unknown-linux-musl"
        else
            TARGET="${ARCH}-unknown-linux-gnu"
        fi
        ;;
    macos)
        [ "$STATIC" -eq 1 ] && err "--static is only available on Linux"
        TARGET="${ARCH}-apple-darwin"
        ;;
esac

if [ "$VERSION" = "latest" ]; then
    log "Resolving latest release..."
    RELEASE_JSON="$(fetch_stdout "https://api.github.com/repos/${REPO}/releases/latest")"
    TAG="$(printf '%s\n' "$RELEASE_JSON" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')"
    [ -n "$TAG" ] || err "could not resolve latest release version"
    VERSION="${TAG#v}"
fi

ARCHIVE="helix-trainer-v${VERSION}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

log "Downloading ${ARCHIVE}..."
fetch "${BASE_URL}/${ARCHIVE}" "${TMP_DIR}/${ARCHIVE}"
fetch "${BASE_URL}/${ARCHIVE}.sha256" "${TMP_DIR}/${ARCHIVE}.sha256"

log "Verifying checksum..."
verify_checksum "$TMP_DIR" "$ARCHIVE" || err "checksum mismatch"

if resolve_exarch; then
    log "Extracting (via exarch)..."
    "$EXARCH_BIN" extract --quiet "${TMP_DIR}/${ARCHIVE}" "$TMP_DIR"
else
    log "Extracting..."
    tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "$TMP_DIR"
fi

mkdir -p "$INSTALL_DIR"
install -m 755 "${TMP_DIR}/helix-trainer-v${VERSION}-${TARGET}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

log "Installed ${BINARY} ${VERSION} to ${INSTALL_DIR}/${BINARY}"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        log ""
        log "warning: ${INSTALL_DIR} is not on your PATH."
        log "Add it, e.g.: export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

log ""
log "Run it with: ${BINARY}"
