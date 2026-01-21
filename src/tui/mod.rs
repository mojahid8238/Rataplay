use crate::app::{App, AppState};
use crate::model::download::DownloadStatus;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
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

pub fn ui(f: &mut Frame, app: &mut App, picker: &mut Picker) {
    let mut constraints = vec![
        Constraint::Length(3), // Search
        Constraint::Min(0),    // Main
    ];

    if app.playback_title.is_some() {
        constraints.push(Constraint::Length(3)); // Playback Bar
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
        constraints.push(Constraint::Length(1));
    }

    if app.terminal_loading {
        constraints.push(Constraint::Length(3)); // Terminal Loading Bar
    }

    constraints.push(Constraint::Length(3)); // Status Bar

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

    // Render Global Download Progress
    if active_download_count > 0 {
        let clamped_progress = avg_progress.clamp(0.0, 100.0);
        let label = format!(" Downloading {} items: {:.1}% ", active_download_count, clamped_progress);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(app.theme.accent).bg(app.theme.bg))
            .label(label)
            .ratio(clamped_progress / 100.0)
            .use_unicode(true);
        
        let gauge_layout = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ])
            .split(main_layout[current_idx]);

        f.render_widget(gauge, gauge_layout[1]);
        current_idx += 1;
    }

    if app.terminal_loading {
        let status = if app.terminal_loading_error.is_some() { "ERROR" } else { "Loading for Terminal..." };
        render_download_gauge(f, app, app.terminal_loading_progress, status, main_layout[current_idx]);
        current_idx += 1;
    }

    status_bar::render_status_bar(f, app, main_layout[current_idx]);

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