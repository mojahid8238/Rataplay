use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, BorderType, Table, Row, Cell},
    style::{Modifier, Style, Color},
    text::Span,
};

use crate::app::{App, AppState};
use crate::model::download::DownloadStatus;

use super::widgets::{create_progress_bar_string, truncate_str};

pub fn render_downloads_view(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let has_downloads = !app.download_manager.task_order.is_empty();
    
    let chunks = if has_downloads {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), 
                Constraint::Percentage(60), 
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    if has_downloads {
        render_active_downloads(f, app, chunks[0]);
        render_local_files(f, app, chunks[1]);
    } else {
        render_local_files(f, app, chunks[0]);
    }
}

fn render_active_downloads(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("TITLE"),
        Cell::from("SIZE"),
        Cell::from("PROGRESS"),
        Cell::from("SPD"),
        Cell::from("ETA"),
        Cell::from("STATUS"),
    ])
    .style(
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .download_manager
        .task_order
        .iter()
        .enumerate()
        .filter_map(|(i, id)| {
            app.download_manager.tasks.get(id).map(|task| (i, task))
        })
        .map(|(i, task)| {
            let is_focused = app.state == AppState::Downloads && app.selected_download_index == Some(i);
            let indicator = if is_focused { "┃ " } else { "  " };
            let checkbox = if app.selected_download_indices.contains(&i) { "[x] " } else { "[ ] " };

            let status_span = match &task.status {
                DownloadStatus::Downloading => {
                    Span::styled("Downloading", Style::default().fg(Color::Green))
                }
                DownloadStatus::Finished => {
                    Span::styled("Finished", Style::default().fg(Color::Cyan))
                }
                DownloadStatus::Error(e) => {
                    Span::styled(format!("Error: {}", e), Style::default().fg(Color::Red))
                }
                DownloadStatus::Paused => {
                    Span::styled("Paused", Style::default().fg(Color::Yellow))
                }
                DownloadStatus::Canceled => {
                    Span::styled("Canceled", Style::default().fg(Color::DarkGray))
                }
                _ => Span::raw("Pending"),
            };

            let row_style = if is_focused {
                Style::default().bg(app.theme.highlight).fg(app.theme.fg).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let title_avail = (area.width as f64 * 0.3).round() as usize; 
            let display_title = truncate_str(&task.title, title_avail.saturating_sub(6));

            Row::new(vec![
                Cell::from(format!("{}{}{}", indicator, checkbox, display_title)),
                Cell::from(task.total_size.clone()),
                Cell::from(create_progress_bar_string(
                    task.progress,
                    15,
                    app.theme.accent,
                    Color::DarkGray,
                )),
                Cell::from(task.speed.clone()),
                Cell::from(task.eta.clone()),
                Cell::from(status_span),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(10),
            Constraint::Percentage(25),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Active Tasks "),
    );
    
    app.downloads_active_state.select(if app.state == AppState::Downloads { app.selected_download_index } else { None });
    f.render_stateful_widget(table, area, &mut app.downloads_active_state);
}

fn render_local_files(f: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("FILENAME"),
        Cell::from("SIZE"),
        Cell::from("FORMAT"),
        Cell::from("STATUS"),
    ])
    .style(
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )
    .height(1)
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .local_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let is_focused = app.state == AppState::Downloads && app.selected_local_file_index == Some(i);
            let is_selected = app.selected_local_file_indices.contains(&i);
            
            let indicator = if is_focused { "┃ " } else { "  " };
            let checkbox = if is_selected { "[x] " } else { "[ ] " };

            let status_span = if file.is_garbage {
                 Span::styled("Incomplete/Temp", Style::default().fg(Color::Yellow))
            } else {
                 Span::styled("Downloaded", Style::default().fg(Color::Green))
            };

            let row_style = if is_focused {
                Style::default().bg(app.theme.highlight).fg(app.theme.fg).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let name_avail = (area.width as f64 * 0.5).round() as usize;
            let display_name = truncate_str(&file.name, name_avail.saturating_sub(6));

            Row::new(vec![
                Cell::from(format!("{}{}{}", indicator, checkbox, display_name)),
                Cell::from(file.size.clone()),
                Cell::from(file.extension.clone()),
                Cell::from(status_span),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                        .title(" Local Files "),
                );
            
                app.downloads_local_state.select(if app.state == AppState::Downloads { app.selected_local_file_index } else { None });
                f.render_stateful_widget(table, area, &mut app.downloads_local_state);
            }