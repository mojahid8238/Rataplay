use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Modifier},
    widgets::Widget,
};
use super::theme::Theme;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationMode {
    Wave,
    Breathe,
    Glitch,
    Neon,
    Static,
}

impl AnimationMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Wave => "Wave",
            Self::Breathe => "Breathe",
            Self::Glitch => "Glitch",
            Self::Neon => "Neon",
            Self::Static => "Static",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::Wave, Self::Breathe, Self::Glitch, Self::Neon, Self::Static]
    }
}

pub struct Logo {
    frame_index: usize,
    theme: Theme,
    mode: AnimationMode,
}

impl Logo {
    pub fn new(frame_index: usize, theme: Theme, mode: AnimationMode) -> Self {
        Self { frame_index, theme, mode }
    }
}

const LOGO_BANNER: &[&str] = &[
    r" ____       _              _             ",
    r"|  _ \ __ _| |_ __ _ _ __ | | __ _ _   _ ",
    r"| |_) / _` | __/ _` | '_ \| |/ _` | | | |",
    r"|  _ < (_| | || (_| | |_) | | (_| | |_| |",
    r"|_| \_\__,_|\__\__,_| .__/|_|\__,_|\__, |",
    r"                    |_|            |___/ ",
];

const MESSAGES: &[&str] = &[
    "Welcome to Rataplay!",
    "Double-click a video to see actions!",
    "Press Ctrl+t to change themes!",
    "Press Ctrl+a to toggle animations!",
    "Press Ctrl+l to toggle live streams!",
    "Press Ctrl+p to toggle playlists!",
    "Press Ctrl+s to open settings!",
    "I can download entire playlists!",
    "Try watching videos in the terminal!",
    "You can paste URLs directly!",
    "Search results are fully scrollable!",
    "Check out the 'Catppuccin' theme!",
    "I use mpv for high-quality playback!",
    "Manage your downloads in the 'd' panel!",
    "I'm written in Rust for speed!",
];

impl Widget for Logo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let msg_index = (self.frame_index / 20) % MESSAGES.len();
        let message = MESSAGES[msg_index];

        let logo_height = LOGO_BANNER.len() as u16;
        let logo_width = LOGO_BANNER.iter().map(|l| l.len()).max().unwrap_or(0) as u16;

        let logo_x = area.x + (area.width.saturating_sub(logo_width)) / 2;
        let logo_y = area.y + (area.height.saturating_sub(logo_height + 4)) / 2 + 1;

        for (row_idx, line) in LOGO_BANNER.iter().enumerate() {
            for (col_idx, ch) in line.chars().enumerate() {
                if ch == ' ' {
                    continue;
                }

                let mut x_offset: i16 = 0;
                let mut y_offset: i16 = 0;
                let mut color_shift = col_idx;

                match self.mode {
                    AnimationMode::Wave => {
                        y_offset = match ((self.frame_index + col_idx / 4) / 2) % 4 {
                            1 => 1, 3 => -1, _ => 0
                        };
                    }
                    AnimationMode::Breathe => {
                        let breathe = (self.frame_index / 4) % 8;
                        y_offset = if breathe < 4 { 1 } else { 0 };
                        color_shift = self.frame_index / 2;
                    }
                    AnimationMode::Glitch => {
                        if (self.frame_index + row_idx) % 15 == 0 {
                            x_offset = (self.frame_index % 3) as i16 - 1;
                        }
                    }
                    AnimationMode::Neon => {
                        color_shift = col_idx + row_idx * 2 + (self.frame_index * 3);
                    }
                    AnimationMode::Static => {}
                }

                let x = (logo_x + col_idx as u16).saturating_add_signed(x_offset);
                let y = (logo_y + row_idx as u16).saturating_add_signed(y_offset);

                if x < area.right() && y < area.bottom() && y >= area.top() {
                    let color_step = (self.frame_index + color_shift / 2) % 12;
                    let color = if color_step < 4 {
                        self.theme.highlight
                    } else if color_step < 8 {
                        self.theme.accent
                    } else {
                        self.theme.fg
                    };

                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_symbol(&ch.to_string())
                            .set_style(Style::default().fg(color).add_modifier(Modifier::BOLD));
                    }
                }
            }
        }

        let bubble_x = area.x + (area.width.saturating_sub(message.len() as u16 + 4)) / 2;
        let bubble_y = logo_y + logo_height + 2;

        if bubble_y < area.bottom() {
            buf.set_string(logo_x + (logo_width / 2), bubble_y - 1, "", Style::default().fg(Color::Yellow));
            let bubble_text = format!("( {} )", message);
            buf.set_string(bubble_x, bubble_y, &bubble_text, Style::default().fg(Color::Yellow));
        }
    }
}