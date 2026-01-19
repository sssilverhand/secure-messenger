# PrivMsg - Private Self-Hosted Messenger

**PrivMsg** is a private, end-to-end encrypted messenger designed for self-hosted deployment. It provides secure communication without relying on third-party services.

## Features

- **End-to-End Encryption**: X25519 key exchange + AES-256-GCM encryption
- **Self-Hosted**: Full control over your data and server
- **Multi-Platform**: Windows, Linux, Android clients
- **Voice & Video Calls**: WebRTC-based calls with TURN server support
- **Voice & Video Messages**: Send encrypted media messages
- **File Transfer**: Encrypted file sharing
- **No Phone/Email Required**: Anonymous access keys for authentication

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    CLIENT APPLICATIONS                      │
├─────────────────────────────────────────────────────────────┤
│
│  ┌──────────────────┐      ┌──────────────────┐
│  │ Desktop Client   │      │  Android Client  │
│  │   (Rust/iced)    │      │    (Kotlin)      │
│  └────────┬─────────┘      └────────┬─────────┘
│           │                         │
│           │  E2EE Messages         │
│           │  (X25519 + AES-256)    │
│           │                         │
└───────────┼─────────────────────────┼─────────────────────────┘
            │                         │
            └───────────┬─────────────┘
                        │
        ┌───────────────▼────────────────┐
        │   PrivMsg Server (Rust/Axum)   │
        │   • Message Relay              │
        │   • User Management            │
        │   • File Storage               │
        └────────────────────────────────┘
                        │
            ┌───────────▼───────────┐
            │   TURN Server         │
            │   (coturn)            │
            │   WebRTC Media Relay  │
            └───────────────────────┘
```

## Quick Start

### Prerequisites

- Docker and Docker Compose
- A VPS with a public IP (for deployment)
- Domain name (optional, but recommended for HTTPS)

### 1. Clone the Repository

```bash
git clone https://github.com/sssilverhand/secure-messenger.git
cd secure-messenger
```

### 2. Configure the Server

Edit `config/server/config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8443

[admin]
master_key = "YOUR_SECURE_ADMIN_KEY"  # Generate a secure key!

[turn]
credential = "YOUR_TURN_SECRET"       # Generate a secure secret!
```

Generate secure keys:
```bash
# Generate admin key
openssl rand -base64 32

# Generate TURN secret
openssl rand -base64 32
```

### 3. Deploy with Docker Compose

```bash
docker-compose up -d
```

This starts:
- PrivMsg Server on port 8443
- TURN Server on ports 3478/5349

### 4. Create User Access Keys

```bash
# Using docker
docker exec -it privmsg-server ./privmsg-server generate-key --admin-key YOUR_ADMIN_KEY

# Output:
# User ID: xxxxxxxx
# Access Key: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Share the User ID and Access Key securely with your users.

### 5. Connect Clients

Download clients from [Releases](../../releases) or build from source.

Configure the client with:
- **Server URL**: `https://your-domain.com:8443` or `http://your-ip:8443`
- **User ID**: From step 4
- **Access Key**: From step 4

---

## Deployment Guide

### VPS Requirements

| Requirement | Minimum | Recommended |
|-------------|---------|-------------|
| RAM | 512 MB | 2 GB |
| CPU | 1 vCPU | 2 vCPU |
| Storage | 5 GB SSD | 20 GB SSD |
| OS | Debian 12+ / Ubuntu 22.04+ | Ubuntu 24.04 |
| Ports | 8443, 3478, 5349, 49152-49200 | Same |

### Step-by-Step VPS Deployment

#### 1. Install Docker

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER

# Install Docker Compose
sudo apt install docker-compose-plugin -y

# Logout and login again for group changes
```

#### 2. Clone and Configure

```bash
# Clone repository
git clone https://github.com/sssilverhand/secure-messenger.git
cd secure-messenger

# Generate secure keys
ADMIN_KEY=$(openssl rand -base64 32)
TURN_SECRET=$(openssl rand -base64 32)

echo "Admin Key: $ADMIN_KEY"
echo "TURN Secret: $TURN_SECRET"

# Edit config
nano config/server/config.toml
```

Update the following in `config/server/config.toml`:
```toml
[admin]
master_key = "YOUR_GENERATED_ADMIN_KEY"

[turn]
credential = "YOUR_GENERATED_TURN_SECRET"
```

Update `config/coturn/turnserver.conf`:
```
static-auth-secret=YOUR_GENERATED_TURN_SECRET
realm=your-domain.com
```

#### 3. Configure Firewall

```bash
# UFW (Ubuntu)
sudo ufw allow 8443/tcp    # API Server
sudo ufw allow 3478/tcp    # TURN TCP
sudo ufw allow 3478/udp    # TURN UDP
sudo ufw allow 5349/tcp    # TURNS TCP
sudo ufw allow 5349/udp    # TURNS UDP
sudo ufw allow 49152:49200/udp  # TURN Media Range
sudo ufw enable
```

#### 4. Start Services

```bash
docker compose up -d
```

#### 5. Verify Deployment

```bash
# Check health
curl http://localhost:8443/health

# Check logs
docker compose logs -f
```

### HTTPS with Let's Encrypt (Recommended)

#### Option 1: Using Nginx Reverse Proxy

```bash
# Install Nginx and Certbot
sudo apt install nginx certbot python3-certbot-nginx -y

# Create Nginx config
sudo nano /etc/nginx/sites-available/privmsg
```

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8443;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/privmsg /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx

# Get SSL certificate
sudo certbot --nginx -d your-domain.com
```

#### Option 2: Direct TLS in Server

1. Obtain certificates (e.g., via certbot)
2. Uncomment TLS section in `config/server/config.toml`:
```toml
[tls]
cert_path = "/app/certs/fullchain.pem"
key_path = "/app/certs/privkey.pem"
```
3. Mount certificates in `docker-compose.yml`

---

## Building from Source

### Server

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build server
cd server
cargo build --release

# Binary at: target/release/privmsg-server
```

### Desktop Client (Linux)

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt install build-essential pkg-config libssl-dev libgtk-3-dev \
    libasound2-dev cmake

# Build
cd desktop
cargo build --release

# Binary at: target/release/privmsg-desktop
```

### Desktop Client (Windows)

Requirements:
- Visual Studio Build Tools 2022 with C++ workload
- Rust (via rustup)

```powershell
cd desktop
cargo build --release

# Binary at: target\release\privmsg-desktop.exe
```

### Android Client

Requirements:
- Android Studio or command line SDK
- JDK 17+

```bash
cd android
./gradlew assembleRelease

# APK at: app/build/outputs/apk/release/app-release.apk
```

---

## Server CLI Reference

```bash
# Run server
./privmsg-server run -c config.toml

# Generate new user
./privmsg-server generate-key --admin-key YOUR_ADMIN_KEY

# Generate user with specific ID
./privmsg-server generate-key --admin-key YOUR_ADMIN_KEY --user-id custom_id

# List all users
./privmsg-server list-keys --admin-key YOUR_ADMIN_KEY

# Revoke user access
./privmsg-server revoke-key --admin-key YOUR_ADMIN_KEY --user-id USER_ID
```

---

## API Reference

### Authentication

#### Login
```bash
POST /api/v1/auth/login
Content-Type: application/json

{
  "user_id": "xxxxxxxx",
  "access_key": "...",
  "device_name": "My Phone",
  "device_type": "android",
  "device_public_key": "base64_encoded_key"
}

Response:
{
  "token": "session_token",
  "device_id": "device_id",
  "expires_at": 1234567890,
  "user": { ... }
}
```

### Admin Operations

#### Create User
```bash
POST /api/v1/admin/users
Content-Type: application/json

{
  "admin_key": "YOUR_ADMIN_KEY"
}

Response:
{
  "user_id": "xxxxxxxx",
  "access_key": "..."
}
```

#### Get Server Stats
```bash
GET /api/v1/admin/stats
Content-Type: application/json

{
  "admin_key": "YOUR_ADMIN_KEY"
}
```

### WebSocket

Connect to `/ws` for real-time messaging.

```javascript
// Authenticate after connecting
{
  "type": "authenticate",
  "payload": { "token": "session_token" }
}

// Send message
{
  "type": "message",
  "payload": {
    "message_id": "uuid",
    "sender_id": "...",
    "recipient_id": "...",
    "encrypted_content": "base64_encrypted_data",
    "message_type": "text",
    "timestamp": 1234567890
  }
}
```

---

## Security

### Encryption

- **Key Exchange**: X25519 Elliptic Curve Diffie-Hellman
- **Message Encryption**: AES-256-GCM
- **Server cannot read messages**: All encryption happens on clients

### Authentication

- **Access Keys**: 32-byte random, base64url encoded
- **Sessions**: Token-based with 30-day expiry
- **No personal data required**: No phone, email, or identity verification

### Best Practices

1. **Use HTTPS** in production
2. **Change default secrets** before deployment
3. **Keep server updated** for security patches
4. **Backup database** regularly
5. **Monitor logs** for suspicious activity

---

## Troubleshooting

### Server won't start
```bash
# Check logs
docker compose logs privmsg-server

# Common issues:
# - Port already in use
# - Invalid config file
# - Database permission issues
```

### WebRTC calls not working
```bash
# Verify TURN server is running
docker compose logs coturn

# Check firewall ports
sudo ufw status

# Verify TURN credentials match between server and coturn config
```

### Client can't connect
- Verify server URL is correct
- Check if ports are open: `nc -zv your-server.com 8443`
- For HTTPS, ensure certificate is valid

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

---

## Support

- **Issues**: [GitHub Issues](../../issues)
- **Discussions**: [GitHub Discussions](../../discussions)

---

*PrivMsg - Your private communication, your rules.*
