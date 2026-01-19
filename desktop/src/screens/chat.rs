//! Chat screen for PrivMsg Desktop

use crate::messages::Message;
use crate::state::{AppState, ChatMessage, MessageStatus, MessageType};
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column, Space,
};
use iced::{Alignment, Element, Length};

pub struct ChatScreen;

impl ChatScreen {
    pub fn view(state: &AppState, peer_id: &str) -> Element<'static, Message> {
        // Header
        let header = Self::header(state, peer_id);

        // Messages
        let messages = Self::messages_view(state);

        // Input area
        let input = Self::input_area(state);

        // Main layout
        let content = column![header, messages, input]
            .width(Length::Fill)
            .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn header(state: &AppState, peer_id: &str) -> Element<'static, Message> {
        // Back button
        let back_btn = button(text("<").size(20))
            .padding([8, 14])
            .on_press(Message::GoBack);

        // Peer info
        let conv = state.conversations.iter().find(|c| c.peer_id == peer_id);
        let name = conv
            .and_then(|c| c.peer_name.as_deref())
            .unwrap_or(peer_id);
        let first_char = name.chars().next().unwrap_or('?').to_uppercase().to_string();

        let avatar = container(
            text(&first_char)
                .size(16)
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .width(40)
        .height(40)
        .center_x()
        .center_y();

        let peer_info = column![text(name).size(16), text("online").size(12),].spacing(2);

        // Call buttons
        let voice_call_btn = button(text("Call").size(12))
            .padding(8)
            .on_press(Message::StartCall(peer_id.to_string(), false));

        let video_call_btn = button(text("Video").size(12))
            .padding(8)
            .on_press(Message::StartCall(peer_id.to_string(), true));

        row![
            back_btn,
            Space::with_width(8),
            avatar,
            Space::with_width(12),
            peer_info,
            Space::with_width(Length::Fill),
            voice_call_btn,
            Space::with_width(8),
            video_call_btn,
        ]
        .padding(12)
        .align_items(Alignment::Center)
        .into()
    }

    fn messages_view(state: &AppState) -> Element<'static, Message> {
        if state.current_messages.is_empty() {
            return container(
                column![
                    text("No messages yet").size(18),
                    Space::with_height(10),
                    text("Send a message to start the conversation").size(14),
                ]
                .align_items(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into();
        }

        let messages: Vec<Element<'static, Message>> = state
            .current_messages
            .iter()
            .map(|msg| Self::message_bubble(msg))
            .collect();

        scrollable(
            Column::with_children(messages)
                .spacing(8)
                .padding(16)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    }

    fn message_bubble(msg: &ChatMessage) -> Element<'static, Message> {
        let is_outgoing = msg.is_outgoing;

        // Message content based on type
        let content = match msg.message_type {
            MessageType::Text => Self::text_message_content(msg),
            MessageType::Voice => Self::voice_message_content(msg),
            MessageType::Video => Self::video_message_content(msg),
            MessageType::Image => Self::image_message_content(msg),
            MessageType::File => Self::file_message_content(msg),
        };

        // Time and status
        let time = AppState::format_timestamp(msg.timestamp);
        let status_icon = if is_outgoing {
            match msg.status {
                MessageStatus::Pending => "...",
                MessageStatus::Sent => "v",
                MessageStatus::Delivered => "vv",
                MessageStatus::Read => "vv",
                MessageStatus::Failed => "!",
            }
        } else {
            ""
        };

        let time_row = row![text(&time).size(11), Space::with_width(4), text(status_icon).size(11),]
            .align_items(Alignment::Center);

        let bubble_content = column![content, time_row]
            .spacing(4)
            .align_items(if is_outgoing {
                Alignment::End
            } else {
                Alignment::Start
            });

        let bubble = container(bubble_content)
            .padding(12)
            .max_width(500);

        // Align left or right based on sender
        let bubble_row = if is_outgoing {
            row![Space::with_width(Length::FillPortion(1)), bubble]
        } else {
            row![bubble, Space::with_width(Length::FillPortion(1))]
        };

        bubble_row.width(Length::Fill).into()
    }

    fn text_message_content(msg: &ChatMessage) -> Element<'static, Message> {
        text(&msg.content).size(14).into()
    }

    fn voice_message_content(msg: &ChatMessage) -> Element<'static, Message> {
        let duration = msg
            .attachment
            .as_ref()
            .and_then(|a| a.duration_ms)
            .map(|d| AppState::format_duration(d / 1000))
            .unwrap_or_else(|| "0:00".to_string());

        let file_id = msg
            .attachment
            .as_ref()
            .map(|a| a.file_id.clone())
            .unwrap_or_default();

        row![
            button(text(">").size(16))
                .padding(10)
                .on_press(Message::DownloadFile(file_id, "voice.ogg".to_string())),
            Space::with_width(8),
            column![text("Voice message").size(14), text(&duration).size(12),].spacing(2),
        ]
        .align_items(Alignment::Center)
        .into()
    }

    fn video_message_content(msg: &ChatMessage) -> Element<'static, Message> {
        let duration = msg
            .attachment
            .as_ref()
            .and_then(|a| a.duration_ms)
            .map(|d| AppState::format_duration(d / 1000))
            .unwrap_or_else(|| "0:00".to_string());

        let file_id = msg
            .attachment
            .as_ref()
            .map(|a| a.file_id.clone())
            .unwrap_or_default();
        let file_name = msg
            .attachment
            .as_ref()
            .map(|a| a.file_name.clone())
            .unwrap_or_else(|| "video.mp4".to_string());

        column![
            container(
                column![
                    text("Video Message").size(14),
                    text(&duration).size(12),
                ]
                .align_items(Alignment::Center),
            )
            .width(200)
            .height(200)
            .center_x()
            .center_y(),
            button(text("Download").size(12))
                .padding(8)
                .on_press(Message::DownloadFile(file_id, file_name)),
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn image_message_content(msg: &ChatMessage) -> Element<'static, Message> {
        let file_id = msg
            .attachment
            .as_ref()
            .map(|a| a.file_id.clone())
            .unwrap_or_default();
        let file_name = msg
            .attachment
            .as_ref()
            .map(|a| a.file_name.clone())
            .unwrap_or_else(|| "image.jpg".to_string());

        column![
            container(text("Image").size(14).horizontal_alignment(iced::alignment::Horizontal::Center))
                .width(250)
                .height(200)
                .center_x()
                .center_y(),
            button(text("Download").size(12))
                .padding(8)
                .on_press(Message::DownloadFile(file_id, file_name)),
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn file_message_content(msg: &ChatMessage) -> Element<'static, Message> {
        let (file_id, file_name, file_size) = msg
            .attachment
            .as_ref()
            .map(|a| {
                (
                    a.file_id.clone(),
                    a.file_name.clone(),
                    AppState::format_file_size(a.file_size),
                )
            })
            .unwrap_or_else(|| (String::new(), "file".to_string(), "0 B".to_string()));

        row![
            text("File").size(24),
            Space::with_width(12),
            column![text(&file_name).size(14), text(&file_size).size(12),].spacing(2),
            Space::with_width(12),
            button(text("Download").size(12))
                .padding(8)
                .on_press(Message::DownloadFile(file_id, file_name.clone())),
        ]
        .align_items(Alignment::Center)
        .into()
    }

    fn input_area(state: &AppState) -> Element<'static, Message> {
        // Recording indicator
        if state.is_recording_voice {
            let duration = state
                .recording_start_time
                .map(|start| chrono::Utc::now().timestamp() - start)
                .unwrap_or(0);

            return container(
                row![
                    button(text("Cancel").size(14))
                        .padding(10)
                        .on_press(Message::CancelRecordingVoice),
                    Space::with_width(Length::Fill),
                    text(format!("Recording... {}", AppState::format_duration(duration)))
                        .size(16),
                    Space::with_width(Length::Fill),
                    button(text("Send").size(14))
                        .padding(10)
                        .on_press(Message::StopRecordingVoice),
                ]
                .padding(12)
                .align_items(Alignment::Center),
            )
            .into();
        }

        // Regular input
        let attach_btn = button(text("Attach").size(12))
            .padding(10)
            .on_press(Message::AttachFile);

        let input = text_input("Message", &state.message_input)
            .on_input(Message::MessageInputChanged)
            .on_submit(Message::SendMessage)
            .padding(12)
            .width(Length::Fill);

        let send_or_voice = if state.message_input.trim().is_empty() {
            button(text("Mic").size(12))
                .padding(10)
                .on_press(Message::StartRecordingVoice)
        } else {
            button(text("Send").size(12))
                .padding(10)
                .on_press(Message::SendMessage)
        };

        container(
            row![attach_btn, Space::with_width(8), input, Space::with_width(8), send_or_voice,]
                .padding(12)
                .align_items(Alignment::Center),
        )
        .into()
    }
}
