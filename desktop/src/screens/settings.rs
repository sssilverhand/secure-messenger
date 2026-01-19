//! Settings screen for PrivMsg Desktop

use crate::messages::Message;
use crate::state::AppState;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, text, Space,
};
use iced::{Alignment, Element, Length};

pub struct SettingsScreen;

impl SettingsScreen {
    pub fn view(state: &AppState) -> Element<'static, Message> {
        // Header
        let header = row![
            button(text("<").size(20))
                .padding([8, 14])
                .on_press(Message::GoBack),
            Space::with_width(16),
            text("Settings").size(24),
        ]
        .padding(16)
        .align_items(Alignment::Center);

        // User info section
        let user_section = if let Some(ref session) = state.session {
            column![
                text("Account").size(18),
                Space::with_height(12),
                row![
                    text("User ID:").size(14),
                    Space::with_width(8),
                    text(&session.user_id).size(14),
                ],
                row![
                    text("Device ID:").size(14),
                    Space::with_width(8),
                    text(&session.device_id).size(14),
                ],
                Space::with_height(20),
            ]
            .spacing(8)
        } else {
            column![]
        };

        // Appearance section
        let themes: Vec<String> = vec!["dark".to_string(), "light".to_string()];
        let current_theme = state.config.ui.theme.clone();

        let appearance_section = column![
            text("Appearance").size(18),
            Space::with_height(12),
            row![
                text("Theme:").size(14),
                Space::with_width(12),
                pick_list(themes, Some(current_theme), Message::ThemeChanged)
                    .width(Length::Fixed(150.0)),
            ]
            .align_items(Alignment::Center),
            Space::with_height(20),
        ]
        .spacing(8);

        // Notifications section
        let notifications_section = column![
            text("Notifications").size(18),
            Space::with_height(12),
            checkbox("Enable notifications", state.config.notifications.enabled)
                .on_toggle(Message::NotificationsChanged),
            checkbox("Notification sounds", state.config.notifications.sound)
                .on_toggle(Message::SoundChanged),
            Space::with_height(20),
        ]
        .spacing(8);

        // Server section
        let server_section = column![
            text("Server").size(18),
            Space::with_height(12),
            row![
                text("Address:").size(14),
                Space::with_width(8),
                text(format!(
                    "{}:{}",
                    state.config.server.host, state.config.server.port
                ))
                .size(14),
            ],
            row![
                text("TLS:").size(14),
                Space::with_width(8),
                text(if state.config.server.use_tls {
                    "Enabled"
                } else {
                    "Disabled"
                })
                .size(14),
            ],
            Space::with_height(20),
        ]
        .spacing(8);

        // About section
        let about_section = column![
            text("About").size(18),
            Space::with_height(12),
            row![
                text("Version:").size(14),
                Space::with_width(8),
                text(env!("CARGO_PKG_VERSION")).size(14),
            ],
            Space::with_height(20),
        ]
        .spacing(8);

        // Logout button
        let logout_section = column![
            Space::with_height(20),
            button(
                text("Log Out")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fixed(200.0))
            .padding(12)
            .on_press(Message::Logout),
        ]
        .align_items(Alignment::Center);

        // Main content
        let content = column![
            header,
            container(
                column![
                    user_section,
                    appearance_section,
                    notifications_section,
                    server_section,
                    about_section,
                    logout_section,
                ]
                .padding(20)
                .max_width(600),
            )
            .width(Length::Fill)
            .center_x(),
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
