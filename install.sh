#!/usr/bin/env bash
# Aether Kernel installer — downloads pre-built binaries from GitHub Releases.
# Usage: curl -fsSL https://raw.githubusercontent.com/baiers/aether/main/install.sh | bash

set -euo pipefail

REPO="baiers/aether"
INSTALL_DIR="${AETHER_HOME:-$HOME/.aether}/bin"
VERSION="${AETHER_VERSION:-latest}"

info()  { printf '\033[1;34m%s\033[0m\n' "$*"; }
error() { printf '\033[1;31mError: %s\033[0m\n' "$*" >&2; exit 1; }

# ── Detect platform ──────────────────────────────────────────────────────────

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                *)       error "Unsupported architecture: $arch" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  echo "x86_64-apple-darwin" ;;
                arm64)   echo "aarch64-apple-darwin" ;;
                *)       error "Unsupported architecture: $arch" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            error "Unsupported OS: $os"
            ;;
    esac
}

# ── Resolve version ──────────────────────────────────────────────────────────

resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
            | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')
        [ -n "$VERSION" ] || error "Could not determine latest version"
    fi
    info "Installing Aether $VERSION"
}

# ── Download and install ─────────────────────────────────────────────────────

install() {
    local target archive_ext archive_name url tmp

    target="$(detect_target)"
    info "Detected target: $target"

    if [[ "$target" == *windows* ]]; then
        archive_ext="zip"
    else
        archive_ext="tar.gz"
    fi

    archive_name="aether-${target}.${archive_ext}"
    url="https://github.com/$REPO/releases/download/$VERSION/$archive_name"

    info "Downloading $url"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    curl -fsSL "$url" -o "$tmp/$archive_name"

    info "Extracting to $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"

    if [ "$archive_ext" = "zip" ]; then
        unzip -qo "$tmp/$archive_name" -d "$INSTALL_DIR"
    else
        tar xzf "$tmp/$archive_name" -C "$INSTALL_DIR"
    fi

    chmod +x "$INSTALL_DIR/aether" "$INSTALL_DIR/aether-mcp" "$INSTALL_DIR/aether-api" 2>/dev/null || true

    # ── Add to PATH ──────────────────────────────────────────────────────

    if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
        local shell_rc=""
        case "${SHELL:-}" in
            */zsh)  shell_rc="$HOME/.zshrc" ;;
            */bash) shell_rc="$HOME/.bashrc" ;;
            *)      shell_rc="$HOME/.profile" ;;
        esac

        if [ -n "$shell_rc" ]; then
            echo "" >> "$shell_rc"
            echo "# Aether" >> "$shell_rc"
            echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$shell_rc"
            info "Added $INSTALL_DIR to PATH in $shell_rc"
            info "Run 'source $shell_rc' or restart your shell"
        fi
    fi

    # ── Verify ───────────────────────────────────────────────────────────

    info ""
    info "Aether installed successfully!"
    "$INSTALL_DIR/aether" --version 2>/dev/null || info "(run 'aether --version' after restarting your shell)"
}

resolve_version
install
