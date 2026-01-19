use ratatui::{
    prelude::Rect,
    widgets::{Block, Borders, BorderType, Table, Row, Cell, TableState},
    style::{Modifier, Style},
    layout::Constraint,
};

use super::theme::{THEME_ACCENT, THEME_HIGHLIGHT, THEME_FG, THEME_BG};
use super::widgets::centered_rect;

pub fn render_format_selection(
    f: &mut ratatui::Frame,
    selected_index: Option<usize>,
    formats: &[crate::model::VideoFormat],
    area: Rect,
) {
    let area = centered_rect(40, 30, area);
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .title(" Select Quality ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME_HIGHLIGHT))
        .style(Style::default().bg(THEME_BG));

    let header_style = Style::default()
        .fg(THEME_ACCENT)
        .add_modifier(Modifier::BOLD);

    let header = Row::new(vec![
        Cell::from(" QUALITY"),
        Cell::from(" FORMAT"),
        Cell::from(" SIZE"),
    ])
    .style(header_style)
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = formats
        .iter()
        .map(|fmt| {
            let quality = if fmt.resolution == "audio only" {
                "Audio".to_string()
            } else if fmt.resolution == "unknown" || fmt.resolution.trim().is_empty() {
                if fmt.note.trim().is_empty() {
                    "Unknown".to_string()
                } else {
                    fmt.note.clone()
                }
            } else {
                let height = fmt.resolution.split('x').last().unwrap_or(&fmt.resolution);
                if !height.is_empty() && height.chars().all(|c| c.is_ascii_digit()) {
                    format!("{}p", height)
                } else if !height.is_empty() {
                    height.to_string()
                } else {
                    "Unknown".to_string()
                }
            };

            let size = fmt
                .filesize
                .map(|s| {
                    let mb = s as f64 / 1024.0 / 1024.0;
                    if mb >= 1024.0 {
                        format!("{:.1} GB", mb / 1024.0)
                    } else {
                        format!("{:.1} MB", mb)
                    }
                })
                .unwrap_or_else(|| "N/A".to_string());

            Row::new(vec![
                Cell::from(format!(" {}", quality)),
                Cell::from(fmt.ext.clone()),
                Cell::from(size),
            ])
            .style(Style::default().fg(THEME_FG))
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(
        Style::default()
            .bg(THEME_HIGHLIGHT)
            .fg(THEME_FG)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("┃ ");

    let mut state = TableState::default();
    state.select(selected_index);

    f.render_stateful_widget(table, area, &mut state);
}