use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, Paragraph},
    style::{Modifier, Style},
};

use crate::app::{App, InputMode};
use super::theme::{THEME_ACCENT, THEME_BORDER, THEME_FG, THEME_HIGHLIGHT};

pub fn render_search_bar(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let width = (area.width as usize).saturating_sub(2);
    let scroll = app.cursor_position.saturating_sub(width.saturating_sub(1));
    let display_query: String = app.search_query.chars().skip(scroll).take(width).collect();

    let input = Paragraph::new(display_query.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default().fg(THEME_FG),
            InputMode::Editing => Style::default()
                .fg(THEME_ACCENT)
                .add_modifier(Modifier::BOLD),
            InputMode::Loading => Style::default().fg(THEME_HIGHLIGHT),
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(
                    Style::default().fg(if app.input_mode == InputMode::Editing {
                        THEME_ACCENT
                    } else {
                        THEME_BORDER
                    }),
                )
                .title(" Search / URL "),
        );
    f.render_widget(input, area);

    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((
            area.x + (app.cursor_position.saturating_sub(scroll)) as u16 + 1,
            area.y + 1,
        ));
    }
}