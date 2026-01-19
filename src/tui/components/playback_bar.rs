use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, Paragraph},
    style::{Modifier, Style, Color},
    text::{Line, Span},
};

use crate::app::App;
use super::theme::{THEME_ACCENT, THEME_FG, THEME_HIGHLIGHT};

pub fn render_playback_bar(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let title = app.playback_title.as_deref().unwrap_or("Unknown");
    
    let duration_str = app
        .playback_duration_str
        .as_deref()
        .unwrap_or("00:00/00:00");

    let status_str = if app.is_paused { " PAUSED " } else { " PLAYING " };
    let status_color = if app.is_paused { Color::Gray } else { THEME_ACCENT };

    let overhead = 70; 
    let available_width = area.width.saturating_sub(overhead) as usize;
    let displayed_title = if title.chars().count() > available_width && available_width > 3 {
        format!("{}...", title.chars().take(available_width.saturating_sub(3)).collect::<String>())
    } else {
        title.to_string()
    };

    let p = Paragraph::new(Line::from(vec![
        Span::styled(
            status_str,
            Style::default()
                .fg(Color::Black)
                .bg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("[{}] ", duration_str),
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            displayed_title,
            Style::default().fg(THEME_FG).add_modifier(Modifier::ITALIC),
        ),
        Span::raw(" | "),
        Span::styled(
            "p",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Pause | "),
        Span::styled(
            "Arrows",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Seek | "),
        Span::styled(
            "x",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Stop"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(THEME_ACCENT)),
    );
    f.render_widget(p, area);
}