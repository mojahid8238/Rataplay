use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, List, ListItem, ListState},
    style::{Modifier, Style},
    text::{Line, Span},
};
use crossterm::event::KeyCode;

use crate::app::App;
use super::theme::{THEME_HIGHLIGHT, THEME_FG, THEME_BG};
use super::widgets::centered_rect_fixed;

pub fn render_action_menu(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let actions = crate::app::get_available_actions(app);
    let max_width = actions.iter().map(|a| {
        let key_str = match a.key {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Enter => "Enter".to_string(),
            _ => "".to_string(),
        };
        // " [KEY] Name "
        key_str.len() + a.name.len() + 8
    }).max().unwrap_or(30) as u16;

    let height = (actions.len() + 2) as u16;
    let area = centered_rect_fixed(max_width, height, area);
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .title(" Select Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(THEME_BG).fg(THEME_FG))
        .border_style(Style::default().fg(THEME_HIGHLIGHT));

    let items: Vec<ListItem> = actions
        .iter()
        .map(|action| {
            let key_str = match action.key {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Enter => "Enter".to_string(),
                _ => "".to_string(),
            };
            let content = Line::from(vec![
                Span::styled(
                    format!(" [{}] ", key_str.to_uppercase()),
                    Style::default()
                        .fg(THEME_HIGHLIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(action.name),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(THEME_HIGHLIGHT)
                .fg(THEME_FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("┃ ");

    let mut state = ListState::default();
    f.render_stateful_widget(list, area, &mut state);
}