#!/bin/bash
#
# Build all PrivMsg components
# Server + Desktop + Android (if tools available)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  PrivMsg Complete Build Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Track what we build
BUILT_COMPONENTS=""

# 1. Build Server
echo -e "${GREEN}[1/3] Building Server...${NC}"
if [ -f "$SCRIPT_DIR/build_server.sh" ]; then
    bash "$SCRIPT_DIR/build_server.sh"
    BUILT_COMPONENTS="$BUILT_COMPONENTS Server"
else
    echo -e "${YELLOW}Skipping server (script not found)${NC}"
fi
echo ""

# 2. Build Desktop Client
echo -e "${GREEN}[2/3] Building Desktop Client...${NC}"
if command -v cargo &> /dev/null; then
    bash "$SCRIPT_DIR/build_desktop.sh"
    BUILT_COMPONENTS="$BUILT_COMPONENTS Desktop"
else
    echo -e "${YELLOW}Skipping desktop (Rust not installed)${NC}"
fi
echo ""

# 3. Build Android (optional)
echo -e "${GREEN}[3/3] Building Android APK...${NC}"
if command -v java &> /dev/null; then
    bash "$SCRIPT_DIR/build_android.sh"
    BUILT_COMPONENTS="$BUILT_COMPONENTS Android"
else
    echo -e "${YELLOW}Skipping Android (Java not installed)${NC}"
fi
echo ""

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Build Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# List output files
BUILD_DIR="$PROJECT_DIR/build"
if [ -d "$BUILD_DIR" ]; then
    echo "Output files:"
    find "$BUILD_DIR" -type f \( -name "*.exe" -o -name "*.tar.gz" -o -name "*.zip" -o -name "*.apk" -o -name "privmsg-server" -o -name "privmsg-desktop" \) | while read file; do
        SIZE=$(du -h "$file" | cut -f1)
        echo "  $file ($SIZE)"
    done
fi

echo ""
echo -e "${GREEN}Built components:$BUILT_COMPONENTS${NC}"
echo -e "${BLUE}========================================${NC}"
