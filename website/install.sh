#!/bin/bash
set -e

# Ferrite Installer Script
# Detects OS/Arch and downloads precompiled release binaries.

echo "========================================="
echo " Installing Ferrite Compiler v3.2.0..."
echo "========================================="

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM=linux;;
    Darwin*)    PLATFORM=macos;;
    *)          echo "Error: Unsupported operating system: ${OS}"; exit 1;;
esac

# Detect Architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64*)    TARGET_ARCH=x86_64;;
    arm64*|aarch64*) TARGET_ARCH=arm64;;
    *)          echo "Error: Unsupported architecture: ${ARCH}"; exit 1;;
esac

# Formulate download URL based on GitHub Releases
# Using v3.2.0 release assets
REPO="vishwanathdvgmm/ferrite"
VERSION="v3.2.0"

if [ "${PLATFORM}" = "linux" ]; then
    BINARY_NAME="ferrite-${VERSION}-linux-x86_64.tar.gz"
else
    # macOS
    if [ "${TARGET_ARCH}" = "arm64" ]; then
        BINARY_NAME="ferrite-${VERSION}-macos-arm64.tar.gz"
    else
        BINARY_NAME="ferrite-${VERSION}-macos-x86_64.tar.gz"
    fi
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}"
TEMP_DIR="$(mktemp -d)"

echo "OS: ${PLATFORM} (${TARGET_ARCH})"
echo "Source: ${URL}"
echo "Downloading..."

# Download tarball
if curl -sSfL "${URL}" -o "${TEMP_DIR}/ferrite.tar.gz"; then
    echo "Download completed."
else
    echo "Error: Failed to download binary from GitHub Releases."
    echo "Please verify the releases page at: https://github.com/${REPO}/releases"
    exit 1
fi

# Extract
cd "${TEMP_DIR}"
tar -xzf ferrite.tar.gz

# Find target binary path
INSTALL_DIR="/usr/local/bin"
if [ ! -w "${INSTALL_DIR}" ]; then
    # Fallback to local user path if root is not writable
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "${INSTALL_DIR}"
    echo "Notice: /usr/local/bin is not writable. Installing to local path: ${INSTALL_DIR}"
fi

mv ferrite "${INSTALL_DIR}/ferrite"
chmod +x "${INSTALL_DIR}/ferrite"

echo "========================================="
echo "✅ Ferrite installed successfully!"
echo "Installed path: ${INSTALL_DIR}/ferrite"
echo "========================================="
echo ""
echo "Try running:"
echo "  ferrite --version"
echo ""
