use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, Paragraph},
    style::Style,
};

use crate::app::{App, AppState, InputMode};
use super::theme::{THEME_ACCENT, THEME_BORDER};

pub fn render_status_bar(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let mode_str = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
        InputMode::Loading => "LOADING",
    };

    let key_hints = match app.input_mode {
        InputMode::Normal => {
            match app.state {
                AppState::Downloads => "q: Quit | Tab: Back | d/b: Toggle | j/k: Nav | Space: Select | Enter: Options".to_string(),
                _ => {
                    let tab_hint = if app.show_downloads_panel { " | Tab: Downloads" } else { "" };
                    if !app.playlist_stack.is_empty() {
                        format!("q: Quit | /: Search{} | j/k: Nav | Space: Select | B: Back | Enter: Options", tab_hint)
                    } else {
                        format!("q: Quit | d/b: Toggle | /: Search{} | j/k: Nav | Enter: Open", tab_hint)
                    }
                }
            }
        }
        InputMode::Editing => "Esc: Normal Mode | Enter: Search".to_string(),
        InputMode::Loading => "Please wait...".to_string(),
    };

    let status_msg = app.status_message.as_deref().unwrap_or("");
    let text = if status_msg.is_empty() {
        format!(" [{}] {} ", mode_str, key_hints)
    } else {
        format!(" [{}] {} | {} ", mode_str, key_hints, status_msg)
    };

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_BORDER)),
        )
        .style(Style::default().fg(THEME_ACCENT));
    f.render_widget(p, area);
}