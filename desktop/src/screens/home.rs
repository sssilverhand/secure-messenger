//! Home screen with conversation list for PrivMsg Desktop

use crate::messages::Message;
use crate::state::{AppState, Conversation};
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space, Column,
};
use iced::{Alignment, Element, Length};

pub struct HomeScreen;

impl HomeScreen {
    pub fn view(state: &AppState) -> Element<'static, Message> {
        // Header
        let header = Self::header(state);

        // Search bar (conditional)
        let search = if state.show_search {
            Self::search_bar(state)
        } else {
            column![].into()
        };

        // Conversation list
        let conversations = Self::conversation_list(state);

        // Main layout
        let content = column![header, search, conversations]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn header(state: &AppState) -> Element<'static, Message> {
        let title = text("Chats").size(24);

        let search_btn = button(text("Search").size(14))
            .padding(8)
            .on_press(Message::ToggleSearch);

        let settings_btn = button(text("Settings").size(14))
            .padding(8)
            .on_press(Message::OpenSettings);

        let user_info = if let Some(ref session) = state.session {
            text(&session.user_id).size(12)
        } else {
            text("").size(12)
        };

        row![
            title,
            Space::with_width(Length::Fill),
            user_info,
            Space::with_width(10),
            search_btn,
            Space::with_width(5),
            settings_btn,
        ]
        .padding(16)
        .align_items(Alignment::Center)
        .into()
    }

    fn search_bar(state: &AppState) -> Element<'static, Message> {
        let input = text_input("Search user by ID...", &state.search_query)
            .on_input(Message::SearchQueryChanged)
            .on_submit(Message::SearchUser)
            .padding(12)
            .width(Length::Fill);

        let search_btn = button(text("Find").size(14))
            .padding([12, 20])
            .on_press(Message::SearchUser);

        let close_btn = button(text("X").size(14))
            .padding([12, 14])
            .on_press(Message::ToggleSearch);

        let search_row = row![input, search_btn, close_btn]
            .spacing(8)
            .padding([0, 16, 8, 16]);

        // Found user result
        let result: Element<'static, Message> = if let Some(ref user) = state.found_user {
            let name = user
                .display_name
                .as_deref()
                .unwrap_or(&user.user_id);

            let btn_content: Element<'static, Message> = row![
                text(name).size(16),
                Space::with_width(Length::Fill),
                text("Start Chat").size(12),
            ]
            .align_items(Alignment::Center)
            .into();

            let start_chat_btn = button(btn_content)
                .padding(12)
                .width(Length::Fill)
                .on_press(Message::StartChatWithUser(user.user_id.clone()));

            container(start_chat_btn)
                .padding([0, 16, 8, 16])
                .into()
        } else {
            column![].into()
        };

        column![search_row, result].into()
    }

    fn conversation_list(state: &AppState) -> Element<'static, Message> {
        if state.conversations.is_empty() {
            return container(
                column![
                    text("No conversations yet").size(18),
                    Space::with_height(10),
                    text("Search for a user to start chatting").size(14),
                    Space::with_height(20),
                    button(text("Find Users").size(14))
                        .padding(12)
                        .on_press(Message::ToggleSearch),
                ]
                .align_items(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into();
        }

        let list: Vec<Element<'static, Message>> = state
            .conversations
            .iter()
            .map(|conv| Self::conversation_item(conv))
            .collect();

        scrollable(
            Column::with_children(list)
                .spacing(1)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    fn conversation_item(conv: &Conversation) -> Element<'static, Message> {
        let name = conv.peer_name.as_deref().unwrap_or(&conv.peer_id);
        let first_char = name.chars().next().unwrap_or('?').to_uppercase().to_string();

        // Avatar circle
        let avatar = container(
            text(&first_char)
                .size(18)
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .width(48)
        .height(48)
        .center_x()
        .center_y();

        // Name and last message
        let last_msg = conv.last_message.as_deref().unwrap_or("");
        let last_msg_preview = if last_msg.len() > 40 {
            format!("{}...", &last_msg[..37])
        } else {
            last_msg.to_string()
        };

        let text_column = column![
            text(name).size(16),
            text(last_msg_preview).size(13),
        ]
        .spacing(4);

        // Time and unread badge
        let time_text = if let Some(ts) = conv.last_message_time {
            AppState::format_timestamp(ts)
        } else {
            String::new()
        };

        let time_column = if conv.unread_count > 0 {
            column![
                text(&time_text).size(12),
                container(
                    text(conv.unread_count.to_string())
                        .size(12)
                        .horizontal_alignment(iced::alignment::Horizontal::Center),
                )
                .padding([2, 8])
                .center_x(),
            ]
            .spacing(4)
            .align_items(Alignment::End)
        } else {
            column![text(&time_text).size(12),].align_items(Alignment::End)
        };

        let content = row![
            avatar,
            Space::with_width(12),
            text_column,
            Space::with_width(Length::Fill),
            time_column,
        ]
        .align_items(Alignment::Center)
        .padding(12);

        button(content)
            .width(Length::Fill)
            .padding(0)
            .on_press(Message::OpenChat(conv.peer_id.clone()))
            .into()
    }
}

use crate::state::AppState as AS;
impl AS {
    // Method is defined in state.rs, keeping reference here for clarity
}
