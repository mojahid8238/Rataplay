use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, List, ListItem},
    style::{Modifier, Style},
    text::{Line, Span},
};
use crossterm::event::KeyCode;

use crate::app::App;
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
    app.action_menu_area = Some(area);
    
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .title(" Select Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
        .border_style(Style::default().fg(app.theme.highlight));

    let items: Vec<ListItem> = actions
        .iter()
        .map(|action| {
            let key_str = if action.action == crate::app::AppAction::CopyUrlOrId {
                "C/I".to_string()
            } else {
                match action.key {
                    KeyCode::Char(c) => c.to_string().to_uppercase(),
                    KeyCode::Enter => "ENTER".to_string(),
                    _ => "".to_string(),
                }
            };
            let content = Line::from(vec![
                Span::styled(
                    format!(" [{}] ", key_str),
                    Style::default()
                        .fg(app.theme.highlight)
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
                .bg(app.theme.highlight)
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("┃ ");

    f.render_stateful_widget(list, area, &mut app.action_menu_state);
}