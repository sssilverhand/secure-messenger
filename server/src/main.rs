//! PrivMsg Server - Minimal relay server for private messenger
//!
//! This server handles:
//! - User authentication via access keys
//! - Message relay between clients
//! - Temporary message storage for offline delivery
//! - WebRTC signaling for calls
//! - File transfer relay

mod config;
mod crypto;
mod error;
mod handlers;
mod models;
mod storage;
mod websocket;

use std::sync::Arc;
use axum::{
    routing::{get, post, delete},
    Router,
};
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::storage::Storage;
use crate::websocket::WebSocketManager;

/// PrivMsg Server CLI
#[derive(Parser)]
#[command(name = "privmsg-server")]
#[command(about = "Private messenger relay server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Config file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new access key
    GenerateKey {
        /// Admin master key for authorization
        #[arg(long)]
        admin_key: String,

        /// Optional user ID (will be generated if not provided)
        #[arg(long)]
        user_id: Option<String>,
    },

    /// List all registered keys
    ListKeys {
        /// Admin master key
        #[arg(long)]
        admin_key: String,
    },

    /// Revoke an access key
    RevokeKey {
        /// Admin master key
        #[arg(long)]
        admin_key: String,

        /// User ID to revoke
        #[arg(long)]
        user_id: String,
    },

    /// Run the server
    Run,
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub storage: Arc<Storage>,
    pub ws_manager: Arc<WebSocketManager>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "privmsg_server=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    // Load config
    let config = Config::load(&cli.config).await?;
    let config = Arc::new(config);

    match cli.command.unwrap_or(Commands::Run) {
        Commands::GenerateKey { admin_key, user_id } => {
            generate_key(&config, &admin_key, user_id).await?;
        }
        Commands::ListKeys { admin_key } => {
            list_keys(&config, &admin_key).await?;
        }
        Commands::RevokeKey { admin_key, user_id } => {
            revoke_key(&config, &admin_key, &user_id).await?;
        }
        Commands::Run => {
            run_server(config).await?;
        }
    }

    Ok(())
}

async fn generate_key(config: &Config, admin_key: &str, user_id: Option<String>) -> anyhow::Result<()> {
    if admin_key != config.admin.master_key {
        anyhow::bail!("Invalid admin key");
    }

    let storage = Storage::new(&config.storage.database_path).await?;

    let user_id = user_id.unwrap_or_else(|| crypto::generate_user_id());
    let access_key = crypto::generate_access_key();
    let key_hash = crypto::hash_access_key(&access_key);

    storage.create_user(&user_id, &key_hash).await?;

    println!("=== New Access Key Generated ===");
    println!("User ID: {}", user_id);
    println!("Access Key: {}", access_key);
    println!("================================");
    println!("Share these credentials securely with the user.");
    println!("The access key will NOT be shown again!");

    Ok(())
}

async fn list_keys(config: &Config, admin_key: &str) -> anyhow::Result<()> {
    if admin_key != config.admin.master_key {
        anyhow::bail!("Invalid admin key");
    }

    let storage = Storage::new(&config.storage.database_path).await?;
    let users = storage.list_users().await?;

    println!("=== Registered Users ===");
    for user in users {
        println!("User ID: {} | Created: {} | Active: {}",
            user.user_id,
            user.created_at,
            user.is_active
        );
    }

    Ok(())
}

async fn revoke_key(config: &Config, admin_key: &str, user_id: &str) -> anyhow::Result<()> {
    if admin_key != config.admin.master_key {
        anyhow::bail!("Invalid admin key");
    }

    let storage = Storage::new(&config.storage.database_path).await?;
    storage.deactivate_user(user_id).await?;

    println!("User {} has been deactivated", user_id);

    Ok(())
}

async fn run_server(config: Arc<Config>) -> anyhow::Result<()> {
    tracing::info!("Starting PrivMsg Server v{}", env!("CARGO_PKG_VERSION"));

    // Initialize storage
    let storage = Arc::new(Storage::new(&config.storage.database_path).await?);

    // Initialize WebSocket manager
    let ws_manager = Arc::new(WebSocketManager::new());

    // Create app state
    let storage_for_cleanup = Arc::clone(&storage);
    let state = AppState {
        config: config.clone(),
        storage,
        ws_manager,
    };

    // Build routes
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))

        // Authentication
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/refresh", post(handlers::auth::refresh_token))
        .route("/api/v1/auth/logout", post(handlers::auth::logout))

        // User management
        .route("/api/v1/users/me", get(handlers::users::get_current_user))
        .route("/api/v1/users/:user_id", get(handlers::users::get_user))
        .route("/api/v1/users/me/profile", post(handlers::users::update_profile))
        .route("/api/v1/users/me/devices", get(handlers::users::list_devices))
        .route("/api/v1/users/me/devices/:device_id", delete(handlers::users::remove_device))

        // Messages
        .route("/api/v1/messages/pending", get(handlers::messages::get_pending_messages))
        .route("/api/v1/messages/ack", post(handlers::messages::acknowledge_messages))

        // Files
        .route("/api/v1/files/upload", post(handlers::files::upload_file))
        .route("/api/v1/files/:file_id", get(handlers::files::download_file))
        .route("/api/v1/files/:file_id", delete(handlers::files::delete_file))

        // WebSocket for real-time communication
        .route("/ws", get(handlers::websocket::websocket_handler))

        // Admin routes
        .route("/api/v1/admin/users", post(handlers::admin::create_user))
        .route("/api/v1/admin/users/:user_id", delete(handlers::admin::delete_user))
        .route("/api/v1/admin/stats", get(handlers::admin::get_stats))

        // TURN credentials
        .route("/api/v1/turn/credentials", get(handlers::turn::get_credentials))

        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr).await?;

    // Start cleanup task
    let cleanup_interval = config.storage.cleanup_interval_minutes;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(cleanup_interval * 60)
        );
        loop {
            interval.tick().await;
            match storage_for_cleanup.cleanup_expired().await {
                Ok((msgs, files)) => {
                    if msgs > 0 || files > 0 {
                        tracing::info!("Cleanup: removed {} messages, {} files", msgs, files);
                    }
                }
                Err(e) => {
                    tracing::error!("Cleanup failed: {}", e);
                }
            }
        }
    });

    axum::serve(listener, app).await?;

    Ok(())
}
