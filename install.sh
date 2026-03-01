#!/bin/sh
# deadbranch installer script
# Usage: curl -sSf https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/install.sh | sh
#
# This script detects your OS and architecture, downloads the appropriate
# pre-built binary, and installs it to ~/.local/bin (or /usr/local/bin with sudo).

set -e

REPO="armgabrielyan/deadbranch"
BINARY_NAME="deadbranch"
INSTALL_DIR="${DEADBRANCH_INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
    printf "${BLUE}info:${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}success:${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}error:${NC} %s\n" "$1" >&2
    exit 1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)       error "Unsupported operating system: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get the target triple for the current platform
get_target() {
    local os="$1"
    local arch="$2"

    case "$os" in
        linux)
            # Check if we're on musl (Alpine, etc.)
            if ldd --version 2>&1 | grep -q musl; then
                echo "${arch}-unknown-linux-musl"
            else
                echo "${arch}-unknown-linux-gnu"
            fi
            ;;
        macos)
            echo "${arch}-apple-darwin"
            ;;
        windows)
            echo "${arch}-pc-windows-msvc"
            ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    local version
    version=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')

    if [ -z "$version" ]; then
        error "Failed to fetch latest version from GitHub"
    fi

    echo "$version"
}

# Download and install the binary
install() {
    local os arch target version archive_ext archive_name download_url tmp_dir

    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "$os" "$arch")

    info "Detected platform: $os ($arch)"
    info "Target: $target"

    # Get latest version
    info "Fetching latest version..."
    version=$(get_latest_version)
    info "Latest version: v$version"

    # Determine archive extension
    if [ "$os" = "windows" ]; then
        archive_ext="zip"
    else
        archive_ext="tar.gz"
    fi

    archive_name="deadbranch-${version}-${target}.${archive_ext}"
    download_url="https://github.com/${REPO}/releases/download/v${version}/${archive_name}"

    info "Downloading $archive_name..."

    # Create temporary directory
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    # Download the archive
    if ! curl -sSfL "$download_url" -o "$tmp_dir/$archive_name"; then
        error "Failed to download from $download_url"
    fi

    # Extract the archive
    info "Extracting..."
    cd "$tmp_dir"

    if [ "$archive_ext" = "zip" ]; then
        unzip -q "$archive_name"
    else
        tar -xzf "$archive_name"
    fi

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating directory $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi

    # Install the binary
    info "Installing to $INSTALL_DIR/$BINARY_NAME"

    if [ -w "$INSTALL_DIR" ]; then
        cp "$BINARY_NAME" "$INSTALL_DIR/"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
    else
        warn "Cannot write to $INSTALL_DIR, trying with sudo..."
        sudo cp "$BINARY_NAME" "$INSTALL_DIR/"
        sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    fi

    success "deadbranch v$version installed successfully!"

    # Check if install directory is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "Run 'deadbranch --help' to get started"
            ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "Add it to your PATH by adding this line to your shell config:"
            echo ""
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
            echo ""
            ;;
    esac
}

# Check for required commands
check_requirements() {
    if ! command -v curl >/dev/null 2>&1; then
        error "curl is required but not installed"
    fi

    if ! command -v tar >/dev/null 2>&1; then
        error "tar is required but not installed"
    fi
}

# Main
main() {
    echo ""
    echo "  ╭──────────────────────────────────────╮"
    echo "  │  deadbranch installer                │"
    echo "  │  Clean up stale git branches safely  │"
    echo "  ╰──────────────────────────────────────╯"
    echo ""

    check_requirements
    install
}

main "$@"
