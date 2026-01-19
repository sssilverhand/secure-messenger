#!/bin/bash
#
# Build PrivMsg Desktop Client for Linux
# No IDE required - only Rust toolchain
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DESKTOP_DIR="$PROJECT_DIR/desktop"
OUTPUT_DIR="$PROJECT_DIR/build/desktop"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== PrivMsg Desktop Build Script ===${NC}"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust is not installed${NC}"
    echo "Install Rust from https://rustup.rs"
    echo "Or run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Rust version
RUST_VERSION=$(rustc --version | awk '{print $2}')
echo -e "${YELLOW}Rust version: $RUST_VERSION${NC}"

# Install dependencies for Linux
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo -e "${YELLOW}Checking Linux dependencies...${NC}"

    # Debian/Ubuntu
    if command -v apt-get &> /dev/null; then
        # Check if packages are installed, install if missing
        DEPS="build-essential pkg-config libssl-dev libasound2-dev libgtk-3-dev libxdo-dev"
        MISSING=""
        for dep in $DEPS; do
            if ! dpkg -s "$dep" &> /dev/null 2>&1; then
                MISSING="$MISSING $dep"
            fi
        done
        if [ -n "$MISSING" ]; then
            echo -e "${YELLOW}Installing missing dependencies:$MISSING${NC}"
            sudo apt-get update
            sudo apt-get install -y $MISSING
        fi
    fi

    # Arch Linux
    if command -v pacman &> /dev/null; then
        DEPS="base-devel openssl alsa-lib gtk3 libxdo"
        MISSING=""
        for dep in $DEPS; do
            if ! pacman -Qi "$dep" &> /dev/null 2>&1; then
                MISSING="$MISSING $dep"
            fi
        done
        if [ -n "$MISSING" ]; then
            echo -e "${YELLOW}Installing missing dependencies:$MISSING${NC}"
            sudo pacman -S --needed --noconfirm $MISSING
        fi
    fi

    # Fedora
    if command -v dnf &> /dev/null; then
        DEPS="openssl-devel alsa-lib-devel gtk3-devel libxdo-devel"
        sudo dnf install -y $DEPS 2>/dev/null || true
    fi
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build
echo -e "${YELLOW}Building desktop client...${NC}"
cd "$DESKTOP_DIR"

# Release build
cargo build --release

# Copy binary
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cp "$DESKTOP_DIR/target/release/privmsg-desktop" "$OUTPUT_DIR/"
    chmod +x "$OUTPUT_DIR/privmsg-desktop"

    # Create .desktop file for Linux
    cat > "$OUTPUT_DIR/privmsg.desktop" << 'EOF'
[Desktop Entry]
Name=PrivMsg
Comment=Private E2EE Messenger
Exec=privmsg-desktop
Icon=privmsg
Terminal=false
Type=Application
Categories=Network;InstantMessaging;
EOF

    echo -e "${GREEN}Build complete!${NC}"
    echo "Output: $OUTPUT_DIR/privmsg-desktop"

elif [[ "$OSTYPE" == "darwin"* ]]; then
    cp "$DESKTOP_DIR/target/release/privmsg-desktop" "$OUTPUT_DIR/"
    chmod +x "$OUTPUT_DIR/privmsg-desktop"
    echo -e "${GREEN}Build complete!${NC}"
    echo "Output: $OUTPUT_DIR/privmsg-desktop"
fi

# Create archive
echo -e "${YELLOW}Creating archive...${NC}"
cd "$OUTPUT_DIR"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    tar -czf privmsg-linux-$(uname -m).tar.gz privmsg-desktop privmsg.desktop
    echo "Archive: $OUTPUT_DIR/privmsg-linux-$(uname -m).tar.gz"
fi

echo -e "${GREEN}=== Build Complete ===${NC}"
