//! PrivMsg Desktop Client
//!
//! Cross-platform desktop messenger for Windows and Linux.
//! Built with iced GUI framework.

mod app;
mod config;
mod crypto;
mod database;
mod messages;
mod network;
mod screens;
mod state;
mod theme;
mod widgets;

use iced::{Application, Settings, Size};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> iced::Result {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "privmsg_desktop=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting PrivMsg Desktop v{}", env!("CARGO_PKG_VERSION"));

    // Get data directory
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("privmsg");

    std::fs::create_dir_all(&data_dir).ok();

    tracing::info!("Data directory: {:?}", data_dir);

    // Load or create config
    let config = config::AppConfig::load(&data_dir).unwrap_or_default();

    // Run application
    app::PrivMsg::run(Settings {
        window: iced::window::Settings {
            size: Size::new(1200.0, 800.0),
            min_size: Some(Size::new(800.0, 600.0)),
            position: iced::window::Position::Centered,
            ..Default::default()
        },
        default_font: iced::Font::DEFAULT,
        default_text_size: iced::Pixels(14.0),
        antialiasing: true,
        flags: app::Flags {
            data_dir,
            config,
        },
        ..Default::default()
    })
}
