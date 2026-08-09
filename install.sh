#!/bin/sh
# Installs helix-trainer from a pre-built GitHub release binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/bug-ops/helix-trainer/main/install.sh | sh
#   ./install.sh [-v VERSION] [-d DIR] [--static]
#
# Options:
#   -v, --version VERSION   Release to install, e.g. "0.5.12" (default: latest)
#   -d, --dir DIR           Install directory (default: $HOME/.local/bin)
#       --static            Prefer the statically-linked musl build on Linux
#                            (no audio feature)
#   -h, --help              Show this help message
#
# Windows users: run install.ps1 instead.

set -eu

REPO="bug-ops/helix-trainer"
BINARY="helix-trainer"
VERSION="${HELIX_TRAINER_VERSION:-latest}"
INSTALL_DIR="${HELIX_TRAINER_INSTALL_DIR:-$HOME/.local/bin}"
STATIC=0

log() { printf '%s\n' "$*"; }
err() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

usage() {
    sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
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

detect_os() {
    case "$(uname -s)" in
        Linux) echo linux ;;
        Darwin) echo macos ;;
        *) err "unsupported OS: $(uname -s) (use install.ps1 on Windows)" ;;
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
(
    cd "$TMP_DIR"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "${ARCHIVE}.sha256"
    elif command -v shasum >/dev/null 2>&1; then
        EXPECTED="$(awk '{print $1}' "${ARCHIVE}.sha256")"
        ACTUAL="$(shasum -a 256 "${ARCHIVE}" | awk '{print $1}')"
        [ "$EXPECTED" = "$ACTUAL" ] || err "checksum mismatch"
    else
        err "sha256sum or shasum is required to verify the download"
    fi
)

log "Extracting..."
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "$TMP_DIR"

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
