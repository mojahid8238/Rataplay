use ratatui::{
    prelude::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use super::widgets::centered_rect_fixed;
use crate::app::App;
use crate::model::convert::ConvertTarget;

pub fn render_convert_dialog(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let items: Vec<String> = ConvertTarget::ALL
        .iter()
        .map(|target| format!("{:<5} ({})", target.label(), target.description()))
        .collect();

    let max_width = items.iter().map(|s| s.len() + 6).max().unwrap_or(24) as u16;
    let height = (items.len() + 2) as u16;
    let area = centered_rect_fixed(max_width, height, area);
    app.convert_dialog_area = Some(area);

    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .title(" Convert To ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
        .border_style(Style::default().fg(app.theme.highlight));

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|text| ListItem::new(Line::from(Span::raw(text.clone()))))
        .collect();

    let list = List::new(list_items)
        .block(block)
        .highlight_style(app.theme.selected_style())
        .highlight_symbol(app.theme.selected_symbol());

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.convert_dialog_index));
    f.render_stateful_widget(list, area, &mut list_state);
}
