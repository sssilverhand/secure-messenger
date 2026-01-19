#!/bin/bash
# Build script for PrivMsg Server

set -e

echo "==================================="
echo "Building PrivMsg Server"
echo "==================================="

cd "$(dirname "$0")/../server"

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

# Build mode
BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" == "release" ]; then
    echo "Building in release mode..."
    cargo build --release
    BINARY_PATH="target/release/privmsg-server"
else
    echo "Building in debug mode..."
    cargo build
    BINARY_PATH="target/debug/privmsg-server"
fi

# Create output directory
mkdir -p ../dist/server

# Copy binary
if [ -f "$BINARY_PATH" ]; then
    cp "$BINARY_PATH" ../dist/server/
    echo "Binary copied to dist/server/"
elif [ -f "${BINARY_PATH}.exe" ]; then
    cp "${BINARY_PATH}.exe" ../dist/server/
    echo "Binary copied to dist/server/"
fi

# Copy config template
if [ ! -f ../dist/server/config.toml ]; then
    cat > ../dist/server/config.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 8443

[storage]
database_path = "./data/privmsg.db"
files_path = "./data/files"
max_message_age_hours = 168
max_file_age_hours = 72
cleanup_interval_minutes = 60

[turn]
enabled = true
urls = ["turn:your-turn-server.com:3478", "turns:your-turn-server.com:5349"]
username = "privmsg"
credential = "CHANGE_THIS_SECRET"
credential_type = "password"
ttl_seconds = 86400

[admin]
master_key = "CHANGE_THIS_ADMIN_KEY_IMMEDIATELY"

[limits]
max_file_size_mb = 100
max_message_size_kb = 64
max_pending_messages = 10000
rate_limit_messages_per_minute = 120
EOF
    echo "Config template created"
fi

echo ""
echo "Build complete!"
echo ""
echo "To run the server:"
echo "  cd dist/server"
echo "  ./privmsg-server run"
echo ""
echo "IMPORTANT: Edit config.toml and change:"
echo "  - admin.master_key"
echo "  - turn.credential"
echo "  - Set up TLS certificates for production"
