#!/bin/bash
set -e

# Determine operating system and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Set version from environment variable or use default
VERSION="${VERSION:-0.1.0}"

# Create temporary directory
TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# Download URL based on OS and architecture
BINARY_NAME="arch-cli"
# DOWNLOAD_URL="https://github.com/arch-network/arch-cli/releases/${VERSION}/${BINARY_NAME}-${OS}-${ARCH}.tar.gz"
DOWNLOAD_URL="https://raw.githubusercontent.com/Perelyn-Arch/arch-cli/feat/add-install-script/assets/${BINARY_NAME}-${OS}-${ARCH}.tar.gz"

echo "Downloading ${BINARY_NAME} version ${VERSION}..."
curl -sSfL "$DOWNLOAD_URL" -o "$TMP_DIR/${BINARY_NAME}.tar.gz"

# Extract the archive
tar xzf "$TMP_DIR/${BINARY_NAME}.tar.gz" -C "$TMP_DIR"

# Install into /usr/local/bin or similar
INSTALL_DIR="/usr/local/bin"
sudo mv "$TMP_DIR/${BINARY_NAME}" "$INSTALL_DIR/"
sudo chmod +x "$INSTALL_DIR/${BINARY_NAME}"

echo "${BINARY_NAME} ${VERSION} installed successfully to ${INSTALL_DIR}"
