#!/bin/bash
#
# install-release.sh - Build and install claude-workbench release version
#
# Usage: ./scripts/install-release.sh [--local]
#
# Options:
#   --local    Install to ~/.local/bin instead of /usr/local/bin
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="claude-workbench"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$PROJECT_DIR/target/release"

# Default install location
INSTALL_DIR="/usr/local/bin"

# Parse arguments
if [[ "$1" == "--local" ]]; then
    INSTALL_DIR="$HOME/.local/bin"
fi

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Claude Workbench - Release Build & Install         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if we're in the project directory
if [[ ! -f "$PROJECT_DIR/Cargo.toml" ]]; then
    echo -e "${RED}Error: Cargo.toml not found. Run this script from the project root.${NC}"
    exit 1
fi

# Get version from Cargo.toml
VERSION=$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
echo -e "${YELLOW}Version:${NC} $VERSION"
echo -e "${YELLOW}Install to:${NC} $INSTALL_DIR"
echo ""

# Step 1: Build release version
echo -e "${BLUE}[1/3]${NC} Building release version..."
cd "$PROJECT_DIR"
cargo build --release 2>&1 | while read line; do
    echo "      $line"
done

if [[ ! -f "$TARGET_DIR/$BINARY_NAME" ]]; then
    echo -e "${RED}Error: Build failed - binary not found${NC}"
    exit 1
fi

echo -e "${GREEN}      ✓ Build successful${NC}"
echo ""

# Step 2: Create install directory if needed
echo -e "${BLUE}[2/3]${NC} Preparing installation..."

if [[ ! -d "$INSTALL_DIR" ]]; then
    echo "      Creating directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
fi

# Check if we need sudo for /usr/local/bin
NEED_SUDO=false
if [[ "$INSTALL_DIR" == "/usr/local/bin" ]] && [[ ! -w "$INSTALL_DIR" ]]; then
    NEED_SUDO=true
    echo -e "${YELLOW}      Note: Installation to /usr/local/bin requires sudo${NC}"
fi

echo -e "${GREEN}      ✓ Directory ready${NC}"
echo ""

# Step 3: Install binary
echo -e "${BLUE}[3/3]${NC} Installing binary..."

if [[ "$NEED_SUDO" == true ]]; then
    sudo cp "$TARGET_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
else
    cp "$TARGET_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
fi

echo -e "${GREEN}      ✓ Installed to $INSTALL_DIR/$BINARY_NAME${NC}"
echo ""

# Show binary info
BINARY_SIZE=$(ls -lh "$INSTALL_DIR/$BINARY_NAME" | awk '{print $5}')

# Box formatting helper (60 chars inner width)
print_row() {
    local content="$1"
    local padding=$((58 - ${#content}))
    printf "${BLUE}║${NC}  %s%*s${BLUE}║${NC}\n" "$content" "$padding" ""
}

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}                    ${GREEN}Installation Complete${NC}                   ${BLUE}║${NC}"
echo -e "${BLUE}╠════════════════════════════════════════════════════════════╣${NC}"
print_row "Binary:    $BINARY_NAME"
print_row "Version:   $VERSION"
print_row "Size:      $BINARY_SIZE"
print_row "Location:  $INSTALL_DIR/$BINARY_NAME"
echo -e "${BLUE}╠════════════════════════════════════════════════════════════╣${NC}"
printf "${BLUE}║${NC}  Run with:  ${YELLOW}%-45s${NC} ${BLUE} ║${NC}\n" "$BINARY_NAME"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

# Check if install dir is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH${NC}"
    echo -e "Add this to your shell config (~/.zshrc or ~/.bashrc):"
    echo -e "  ${BLUE}export PATH=\"$INSTALL_DIR:\$PATH\"${NC}"
fi