//! Call screen for PrivMsg Desktop

use crate::messages::Message;
use crate::state::{AppState, CallState};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

pub struct CallScreen;

impl CallScreen {
    pub fn view(state: &AppState, peer_id: &str) -> Element<'static, Message> {
        let peer_name = state
            .conversations
            .iter()
            .find(|c| c.peer_id == peer_id)
            .and_then(|c| c.peer_name.as_deref())
            .unwrap_or(peer_id);

        let first_char = peer_name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();

        // Avatar
        let avatar = container(
            text(&first_char)
                .size(64)
                .horizontal_alignment(iced::alignment::Horizontal::Center)
                .vertical_alignment(iced::alignment::Vertical::Center),
        )
        .width(150)
        .height(150)
        .center_x()
        .center_y();

        // Name
        let name = text(peer_name).size(32);

        // Call type
        let call_type = if state.call_is_video {
            "Video Call"
        } else {
            "Voice Call"
        };

        // Status text
        let status = match state.call_state {
            Some(CallState::Outgoing) => "Calling...",
            Some(CallState::Incoming) => "Incoming call",
            Some(CallState::Connecting) => "Connecting...",
            Some(CallState::Connected) => {
                // Show duration
                ""
            }
            _ => "",
        };

        // Duration (if connected)
        let duration = if state.call_state == Some(CallState::Connected) {
            state
                .call_duration
                .map(|d| AppState::format_duration(d))
                .unwrap_or_else(|| "00:00".to_string())
        } else {
            String::new()
        };

        let status_text = if duration.is_empty() {
            text(status).size(18)
        } else {
            text(&duration).size(24)
        };

        // Controls based on call state
        let controls = match state.call_state {
            Some(CallState::Incoming) => Self::incoming_controls(),
            Some(CallState::Outgoing) | Some(CallState::Connecting) => Self::outgoing_controls(),
            Some(CallState::Connected) => Self::connected_controls(state),
            _ => column![].into(),
        };

        // Main layout
        let content = column![
            Space::with_height(Length::FillPortion(1)),
            avatar,
            Space::with_height(30),
            name,
            text(call_type).size(16),
            Space::with_height(10),
            status_text,
            Space::with_height(Length::FillPortion(1)),
            controls,
            Space::with_height(50),
        ]
        .align_items(Alignment::Center)
        .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .into()
    }

    fn incoming_controls() -> Element<'static, Message> {
        row![
            // Decline button
            button(
                container(text("Decline").size(16))
                    .width(80)
                    .height(80)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::RejectCall),
            Space::with_width(60),
            // Accept button
            button(
                container(text("Accept").size(16))
                    .width(80)
                    .height(80)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::AcceptCall),
        ]
        .align_items(Alignment::Center)
        .into()
    }

    fn outgoing_controls() -> Element<'static, Message> {
        // Just end call button
        button(
            container(text("End").size(16))
                .width(80)
                .height(80)
                .center_x()
                .center_y(),
        )
        .on_press(Message::EndCall)
        .into()
    }

    fn connected_controls(state: &AppState) -> Element<'static, Message> {
        let mute_text = if state.call_muted { "Unmute" } else { "Mute" };
        let video_text = if state.call_video_enabled {
            "Video Off"
        } else {
            "Video On"
        };

        let video_element: Element<'static, Message> = if state.call_is_video {
            button(
                container(text(video_text).size(14))
                    .width(70)
                    .height(70)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::ToggleVideo)
            .into()
        } else {
            Space::with_width(0).into()
        };

        row![
            // Mute button
            button(
                container(text(mute_text).size(14))
                    .width(70)
                    .height(70)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::ToggleMute),
            Space::with_width(20),
            // Video toggle (only for video calls)
            video_element,
            Space::with_width(20),
            // End call button
            button(
                container(text("End").size(14))
                    .width(70)
                    .height(70)
                    .center_x()
                    .center_y(),
            )
            .on_press(Message::EndCall),
        ]
        .align_items(Alignment::Center)
        .into()
    }
}
