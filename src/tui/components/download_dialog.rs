use ratatui::{
    prelude::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use super::widgets::centered_rect_fixed;
use crate::app::App;
use crate::app::state::DownloadDialogMode;

pub fn render_download_dialog(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let items = match app.download_dialog_mode {
        DownloadDialogMode::Single => vec!["Video (Best)", "Audio (Best)", "Select Format..."],
        DownloadDialogMode::BulkSelected | DownloadDialogMode::BulkAll => {
            vec!["Video (Best)", "Audio (Best)"]
        }
    };

    let title = match app.download_dialog_mode {
        DownloadDialogMode::Single => " Download As ",
        DownloadDialogMode::BulkSelected => " Download Selected As ",
        DownloadDialogMode::BulkAll => " Download All As ",
    };

    let max_width = items.iter().map(|s| s.len() + 6).max().unwrap_or(24) as u16;

    let height = (items.len() + 2) as u16;
    let area = centered_rect_fixed(max_width, height, area);
    app.download_dialog_area = Some(area);

    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
        .border_style(Style::default().fg(app.theme.highlight));

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|text| ListItem::new(Line::from(Span::raw(*text))))
        .collect();

    let list = List::new(list_items)
        .block(block)
        .highlight_style(app.theme.selected_style())
        .highlight_symbol(app.theme.selected_symbol());

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(app.download_dialog_index));
    f.render_stateful_widget(list, area, &mut list_state);
}
