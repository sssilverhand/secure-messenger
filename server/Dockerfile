# PrivMsg Server Dockerfile
# Multi-stage build for minimal image size

# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies (disable time check for WSL clock skew)
RUN apt-get -o Acquire::Check-Valid-Until=false update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (disable time check for WSL clock skew)
RUN apt-get -o Acquire::Check-Valid-Until=false update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 privmsg

# Copy binary from builder
COPY --from=builder /app/target/release/privmsg-server /app/privmsg-server

# Create data directories
RUN mkdir -p /app/data/files && chown -R privmsg:privmsg /app

# Switch to non-root user
USER privmsg

# Expose port
EXPOSE 8443

# Volume for data persistence
VOLUME ["/app/data"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8443/health || exit 1

# Run server
ENTRYPOINT ["/app/privmsg-server"]
CMD ["run"]
