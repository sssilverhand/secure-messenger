//! Login screen for PrivMsg Desktop

use crate::messages::Message;
use crate::state::AppState;
use iced::widget::{
    button, checkbox, column, container, row, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

pub struct LoginScreen;

impl LoginScreen {
    pub fn view(state: &AppState) -> Element<'static, Message> {
        let title = text("PrivMsg")
            .size(48);

        let subtitle = text("Private Secure Messenger")
            .size(16);

        // Server settings
        let server_section = column![
            text("Server").size(14),
            row![
                text_input("Server address", &state.config.server.host)
                    .on_input(Message::ServerHostChanged)
                    .padding(12)
                    .width(Length::FillPortion(3)),
                text_input("Port", &state.config.server.port.to_string())
                    .on_input(Message::ServerPortChanged)
                    .padding(12)
                    .width(Length::FillPortion(1)),
            ]
            .spacing(10),
            checkbox("Use HTTPS/TLS", state.config.server.use_tls)
                .on_toggle(Message::UseTlsChanged),
        ]
        .spacing(8);

        // Credentials
        let credentials_section = column![
            text("Credentials").size(14),
            text_input("User ID", &state.login_user_id)
                .on_input(Message::UserIdChanged)
                .padding(12),
            text_input("Access Key", &state.login_access_key)
                .on_input(Message::AccessKeyChanged)
                .padding(12)
                .secure(true),
        ]
        .spacing(8);

        // Login button
        let login_btn = if state.is_loading {
            button(
                text("Connecting...")
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(14)
        } else {
            button(
                text("Sign In")
                    .horizontal_alignment(iced::alignment::Horizontal::Center),
            )
            .width(Length::Fill)
            .padding(14)
            .on_press(Message::Login)
        };

        // Form
        let form = column![
            server_section,
            Space::with_height(20),
            credentials_section,
            Space::with_height(20),
            login_btn,
        ]
        .spacing(10)
        .max_width(400);

        // Help text
        let help_text = column![
            text("Need access?").size(12),
            text("Contact your server administrator for credentials.").size(12),
        ]
        .spacing(4)
        .align_items(Alignment::Center);

        // Main layout
        let content = column![
            Space::with_height(Length::FillPortion(1)),
            title,
            subtitle,
            Space::with_height(40),
            form,
            Space::with_height(30),
            help_text,
            Space::with_height(Length::FillPortion(1)),
        ]
        .align_items(Alignment::Center)
        .spacing(10)
        .padding(40);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
