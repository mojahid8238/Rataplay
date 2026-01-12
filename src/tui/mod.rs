use crate::app::{App, AppState, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph},
    Frame,
};
use ratatui_image::picker::Picker;

const THEME_BG: Color = Color::Rgb(20, 20, 25); // Dark slate/blue
const THEME_FG: Color = Color::Rgb(220, 220, 240); // Soft white
const THEME_ACCENT: Color = Color::Rgb(100, 200, 255); // Cyan-ish
const THEME_HIGHLIGHT: Color = Color::Rgb(255, 100, 200); // Pink/Magenta
const THEME_BORDER: Color = Color::Rgb(80, 80, 120); // Muted blue-purple

pub fn ui(f: &mut Frame, app: &mut App, picker: &mut Picker) {
    let constraints = if app.download_progress.is_some() {
        vec![
            Constraint::Length(3), // Search bar
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Download Gauge (increased to 3 to handle borders)
            Constraint::Length(3), // Status bar / Help
        ]
    } else {
        vec![
            Constraint::Length(3), // Search bar
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Status bar / Help
        ]
    };

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1) // Outer margin
        .constraints(constraints)
        .split(f.area());

    // Render Background
    f.render_widget(
        Block::default().style(Style::default().bg(THEME_BG)),
        f.area(),
    );

    render_search_bar(f, app, main_layout[0]);
    render_main_area(f, app, main_layout[1], picker);

    // Download or Status
    if let Some(progress) = app.download_progress {
        render_download_gauge(
            f,
            progress,
            app.download_status.as_deref().unwrap_or("Downloading..."),
            main_layout[2],
        );
        render_status_bar(f, app, main_layout[3]);
    } else {
        render_status_bar(f, app, main_layout[2]);
    }

    if app.state == AppState::ActionMenu {
        render_action_menu(f, app, main_layout[1]);
    }

    if app.state == AppState::FormatSelection {
        render_format_selection(f, app.selected_format_index, &app.formats, f.area());
    }
}

fn render_download_gauge(f: &mut Frame, progress: f32, status: &str, area: Rect) {
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_HIGHLIGHT)),
        )
        .gauge_style(Style::default().fg(THEME_ACCENT).bg(THEME_BG))
        .label(Span::styled(
            format!(" {} {:.0}% ", status, progress * 100.0),
            Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
        ))
        .ratio(progress.into())
        .use_unicode(true);
    f.render_widget(gauge, area);
}

fn render_format_selection(
    f: &mut Frame,
    selected_index: Option<usize>,
    formats: &[crate::model::VideoFormat],
    area: Rect,
) {
    let block = Block::default()
        .title(" Select Format ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME_HIGHLIGHT))
        .style(Style::default().bg(THEME_BG));

    let area = centered_rect(80, 60, area);
    f.render_widget(ratatui::widgets::Clear, area);

    // Create list items
    let items: Vec<ListItem> = formats
        .iter()
        .map(|fmt| {
            let text = format!(
                "{: <10} | {: <5} | {: <12} | {: <20} | {}",
                fmt.format_id,
                fmt.ext,
                fmt.resolution,
                fmt.note,
                fmt.filesize
                    .map(|s| format!("~{}MB", s / 1024 / 1024))
                    .unwrap_or("?".to_string())
            );
            ListItem::new(text)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(THEME_HIGHLIGHT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(selected_index);

    f.render_stateful_widget(list, area, &mut state);
}

fn render_action_menu(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Action Menu ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(THEME_BG).fg(THEME_FG))
        .border_style(Style::default().fg(THEME_HIGHLIGHT));

    let area = centered_rect(50, 40, area);
    f.render_widget(ratatui::widgets::Clear, area);

    let items: Vec<ListItem> = app
        .actions
        .iter()
        .map(|action| {
            let key_str = match action.key {
                crossterm::event::KeyCode::Char(c) => c.to_string(),
                _ => "".to_string(),
            };
            let content = Line::from(vec![
                Span::styled(
                    format!("[{}] ", key_str.to_uppercase()),
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
                .bg(THEME_ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    // In a real app, you might want to manage the selected action index in App state
    // For now, we'll just show the list without a selection.
    // state.select(Some(0)); 

    f.render_stateful_widget(list, area, &mut state);
}


fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let input = Paragraph::new(app.search_query.as_str())
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

    // Show cursor if editing
    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((area.x + app.cursor_position as u16 + 1, area.y + 1));
    }
}

fn render_main_area(f: &mut Frame, app: &mut App, area: Rect, picker: &mut Picker) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    // Left: Results List
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|v| {
            let content = vec![Line::from(vec![
                Span::styled(&v.title, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" ({})", v.duration_string)),
            ])];
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_BORDER))
                .title(" Results "),
        )
        .highlight_style(
            Style::default()
                .bg(THEME_HIGHLIGHT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(app.selected_result_index);

    f.render_stateful_widget(list, chunks[0], &mut state);

    // Right: Details/Preview
    let details_block = Block::default()
        .borders(Borders::ALL)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME_BORDER))
        .title(" Details ");
    let inner_area = details_block.inner(chunks[1]);
    f.render_widget(details_block, chunks[1]);

    if let Some(idx) = app.selected_result_index {
        if let Some(video) = app.search_results.get(idx) {
            // Check for image
            if let Some(img) = app.image_cache.get(&video.id) {
                // Render image
                // We split inner area: Top for image, Bottom for text
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(inner_area);

                // If resize fails or protocol fails, we just don't render or it renders empty/block
                let mut protocol = picker.new_resize_protocol(img.clone());
                let image = ratatui_image::StatefulImage::new(None);
                f.render_stateful_widget(image, layout[0], &mut protocol);

                let text = format!(
                    "Title: {}\nChannel: {}\nDuration: {}\nViews: {}\nUploaded: {}",
                    video.title,
                    video.channel,
                    video.duration_string,
                    video.view_count.unwrap_or(0),
                    video.upload_date.as_deref().unwrap_or("Unknown")
                );
                let p = Paragraph::new(text);
                f.render_widget(p, layout[1]);
            } else {
                // No image yet
                let text = format!(
                    "Title: {}\nChannel: {}\nDuration: {}\nViews: {}\nUploaded: {}\n\n(Loading Thumbnail...)",
                    video.title,
                    video.channel,
                    video.duration_string,
                    video.view_count.unwrap_or(0),
                    video.upload_date.as_deref().unwrap_or("Unknown")
                );
                let p = Paragraph::new(text);
                f.render_widget(p, inner_area);
            }
        }
    } else {
        let p = Paragraph::new("No video selected");
        f.render_widget(p, inner_area);
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
        InputMode::Loading => "LOADING",
    };

    let key_hints = match app.input_mode {
        InputMode::Normal => "q: Quit | /: Search | j/k: Nav | Enter: Select",
        InputMode::Editing => "Esc: Normal Mode | Enter: Search",
        InputMode::Loading => "Please wait... (Searching)",
    };

    let msg = app.status_message.as_deref().unwrap_or("");
    let text = format!(" [{}] {} | {} ", mode_str, key_hints, msg);

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_BORDER)),
        )
        .style(Style::default().fg(THEME_ACCENT));
    f.render_widget(p, area);
}
