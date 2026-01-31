use crate::app::{App, AppState};
use crate::model::download::DownloadStatus;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Color},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::picker::Picker;

pub mod components;
use components::search_bar;
use components::status_bar;
use components::playback_bar;
use components::main_content;
use components::action_menu;
use components::format_selection;
use components::downloads;
use components::settings;

pub fn ui(f: &mut Frame, app: &mut App, picker: &mut Picker) {
    let mut constraints = vec![
        Constraint::Length(3), // Search
        Constraint::Min(0),    // Main
    ];

    if app.playback_title.is_some() {
        constraints.push(Constraint::Length(3)); // Playback Bar
    }

    if app.terminal_loading {
        constraints.push(Constraint::Length(3)); // Terminal Loading Bar
    }

    // Global Download Progress (Combined)
    let (active_download_count, avg_progress) = {
        let tasks: Vec<_> = app.download_manager.tasks.values()
            .filter(|t| matches!(t.status, DownloadStatus::Downloading | DownloadStatus::Pending | DownloadStatus::Paused))
            .collect();
        
        if tasks.is_empty() {
            (0, 0.0)
        } else {
            let total: f64 = tasks.iter().map(|t| t.progress).sum();
            (tasks.len(), total / tasks.len() as f64)
        }
    };
    
    if active_download_count > 0 {
        constraints.push(Constraint::Length(1)); // Thin progress line
        constraints.push(Constraint::Length(2)); // Status Bar (no top border)
    } else {
        constraints.push(Constraint::Length(3)); // Status Bar (all borders)
    }

    let main_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // Render Background
    f.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        f.area(),
    );

    app.search_bar_area = main_layout[0];
    search_bar::render_search_bar(f, app, main_layout[0]);

    let main_content_area;
    let downloads_area;

    if app.show_downloads_panel {
        let content_chunks = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_layout[1]);
        main_content_area = content_chunks[0];
        downloads_area = content_chunks[1];
        
        app.main_content_area = main_content_area;
        app.downloads_area = Some(downloads_area);

        main_content::render_main_area(f, app, main_content_area, picker);
        downloads::render_downloads_view(f, app, downloads_area);
    } else {
        main_content_area = main_layout[1];
        
        app.main_content_area = main_content_area;
        app.downloads_area = None;

        main_content::render_main_area(f, app, main_content_area, picker);
    }

    let mut current_idx = 2;
    if app.playback_title.is_some() {
        app.playback_bar_area = Some(main_layout[current_idx]);
        playback_bar::render_playback_bar(f, app, main_layout[current_idx]);
        current_idx += 1;
    } else {
        app.playback_bar_area = None;
    }

    if app.terminal_loading {
        let status = if app.terminal_loading_error.is_some() { "ERROR" } else { "Loading for Terminal..." };
        render_download_gauge(f, app, app.terminal_loading_progress, status, main_layout[current_idx]);
        current_idx += 1;
    }

    // Render Global Download Progress
    if active_download_count > 0 {
        let clamped_progress = avg_progress.clamp(0.0, 100.0);
        let width = main_layout[current_idx].width as usize;
        let filled_width = (width as f64 * clamped_progress / 100.0).round() as usize;
        let empty_width = width.saturating_sub(filled_width);

        let progress_line = Line::from(vec![
            Span::styled("━".repeat(filled_width), Style::default().fg(app.theme.accent)),
            Span::styled("━".repeat(empty_width), Style::default().fg(Color::DarkGray)),
        ]);
        
        f.render_widget(Paragraph::new(progress_line), main_layout[current_idx]);
        current_idx += 1;
    }

    status_bar::render_status_bar(f, app, main_layout[current_idx], active_download_count > 0);

    if app.state == AppState::ActionMenu {
        app.action_menu_area = Some(main_layout[1]);
        action_menu::render_action_menu(f, app, main_layout[1]);
    } else {
        app.action_menu_area = None;
    }

    if app.state == AppState::FormatSelection {
        app.format_selection_area = Some(f.area());
        format_selection::render_format_selection(f, app, f.area());
    } else {
        app.format_selection_area = None;
    }

    if app.state == AppState::Settings {
        settings::render_settings_menu(f, app, f.area());
    }
}

fn render_download_gauge(f: &mut Frame, app: &App, progress: f32, status: &str, area: Rect) {
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(app.theme.highlight)),
        )
        .gauge_style(Style::default().fg(app.theme.accent).bg(app.theme.bg))
        .label(
            ratatui::text::Span::styled(
                format!(" {} {:.0}% ", status, progress * 100.0),
                Style::default().fg(app.theme.fg).add_modifier(Modifier::BOLD),
            )
        )
        .ratio(progress.into())
        .use_unicode(true);
    f.render_widget(gauge, area);
}