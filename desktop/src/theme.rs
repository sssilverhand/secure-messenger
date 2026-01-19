//! Theme definitions for PrivMsg Desktop

use iced::Color;

pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub surface_secondary: Color,
    pub primary: Color,
    pub primary_hover: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub outgoing_bubble: Color,
    pub incoming_bubble: Color,
    pub online: Color,
    pub border: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::from_rgb(0.11, 0.11, 0.12),      // #1c1c1e
            surface: Color::from_rgb(0.17, 0.17, 0.18),         // #2c2c2e
            surface_secondary: Color::from_rgb(0.22, 0.22, 0.23), // #38383a
            primary: Color::from_rgb(0.0, 0.48, 1.0),           // #007aff
            primary_hover: Color::from_rgb(0.0, 0.58, 1.0),
            text: Color::from_rgb(1.0, 1.0, 1.0),
            text_secondary: Color::from_rgb(0.6, 0.6, 0.6),
            text_tertiary: Color::from_rgb(0.4, 0.4, 0.4),
            success: Color::from_rgb(0.2, 0.78, 0.35),          // #34c759
            warning: Color::from_rgb(1.0, 0.62, 0.04),          // #ff9f0a
            error: Color::from_rgb(1.0, 0.27, 0.23),            // #ff453a
            outgoing_bubble: Color::from_rgb(0.0, 0.48, 1.0),   // #007aff
            incoming_bubble: Color::from_rgb(0.22, 0.22, 0.23), // #38383a
            online: Color::from_rgb(0.2, 0.78, 0.35),           // #34c759
            border: Color::from_rgb(0.3, 0.3, 0.3),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::from_rgb(0.95, 0.95, 0.97),      // #f2f2f7
            surface: Color::from_rgb(1.0, 1.0, 1.0),
            surface_secondary: Color::from_rgb(0.9, 0.9, 0.92),
            primary: Color::from_rgb(0.0, 0.48, 1.0),
            primary_hover: Color::from_rgb(0.0, 0.4, 0.9),
            text: Color::from_rgb(0.0, 0.0, 0.0),
            text_secondary: Color::from_rgb(0.4, 0.4, 0.4),
            text_tertiary: Color::from_rgb(0.6, 0.6, 0.6),
            success: Color::from_rgb(0.2, 0.78, 0.35),
            warning: Color::from_rgb(1.0, 0.62, 0.04),
            error: Color::from_rgb(1.0, 0.23, 0.19),
            outgoing_bubble: Color::from_rgb(0.0, 0.48, 1.0),
            incoming_bubble: Color::from_rgb(0.9, 0.9, 0.92),
            online: Color::from_rgb(0.2, 0.78, 0.35),
            border: Color::from_rgb(0.8, 0.8, 0.8),
        }
    }
}

// Common colors
pub mod colors {
    use iced::Color;

    pub const PRIMARY_BLUE: Color = Color::from_rgb(0.0, 0.48, 1.0);
    pub const GREEN: Color = Color::from_rgb(0.2, 0.78, 0.35);
    pub const RED: Color = Color::from_rgb(1.0, 0.27, 0.23);
    pub const ORANGE: Color = Color::from_rgb(1.0, 0.62, 0.04);
    pub const PURPLE: Color = Color::from_rgb(0.69, 0.32, 0.87);
    pub const GRAY: Color = Color::from_rgb(0.56, 0.56, 0.58);
    pub const WHITE: Color = Color::WHITE;
    pub const BLACK: Color = Color::BLACK;
}
