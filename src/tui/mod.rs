use crate::app::{App, AppState, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::*,
};
use ratatui_image::picker::Picker;

const THEME_BG: Color = Color::Rgb(20, 20, 25); // Dark slate/blue
const THEME_FG: Color = Color::Rgb(220, 220, 240); // Soft white
const THEME_ACCENT: Color = Color::Rgb(100, 200, 255); // Cyan-ish
const THEME_HIGHLIGHT: Color = Color::Rgb(255, 100, 200); // Pink/Magenta
const THEME_BORDER: Color = Color::Rgb(80, 80, 120); // Muted blue-purple

pub fn ui(f: &mut Frame, app: &mut App, picker: &mut Picker) {
    let mut constraints = vec![
        Constraint::Length(3), // Search bar
        Constraint::Min(1),    // Main content
    ];

    if app.playback_title.is_some() {
        constraints.push(Constraint::Length(3)); // Playback info
    }

    // Combined Download Bar
    let active_downloads = app
        .download_manager
        .tasks
        .values()
        .filter(|t| t.status == crate::model::download::DownloadStatus::Downloading)
        .count();

    if active_downloads > 0 {
        constraints.push(Constraint::Length(3)); // Download bar
    }

    if app.terminal_loading {
        constraints.push(Constraint::Length(3)); // Terminal loading progress
    }

    constraints.push(Constraint::Length(3)); // Status bar

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

    // Main content area, potentially split with downloads
    let main_content_area;
    let downloads_area;

    if app.show_downloads_panel {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)]) // 60% for main, 40% for downloads
            .split(main_layout[1]);
        main_content_area = content_chunks[0];
        downloads_area = content_chunks[1];
        render_main_area(f, app, main_content_area, picker);
        render_downloads_view(f, app, downloads_area);
    } else {
        main_content_area = main_layout[1];
        render_main_area(f, app, main_content_area, picker);
    }

    let mut current_idx = 2;
    if app.playback_title.is_some() {
        render_playback_bar(f, app, main_layout[current_idx]);
        current_idx += 1;
    }

    if active_downloads > 0 {
        let total_progress = app
            .download_manager
            .tasks
            .values()
            .filter(|t| t.status == crate::model::download::DownloadStatus::Downloading)
            .map(|t| t.progress as f32)
            .sum::<f32>();
        let avg_progress = total_progress / active_downloads as f32;
        render_download_gauge(
            f,
            avg_progress / 100.0,
            &format!("Downloading {} files...", active_downloads),
            main_layout[current_idx],
        );
        current_idx += 1;
    }

    if app.terminal_loading {
        let status = app
            .terminal_loading_error
            .as_deref()
            .map(|err| format!("Error: {}", err))
            .unwrap_or_else(|| "Buffering Terminal Playback...".to_string());
        render_download_gauge(
            f,
            app.terminal_loading_progress,
            &status,
            main_layout[current_idx],
        );
        current_idx += 1;
    }

    render_status_bar(f, app, main_layout[current_idx]);

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
        .label(
            Span::styled(
                format!(" {} {:.0}% ", status, progress * 100.0),
                Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
            )
        )
        .ratio(progress.into())
        .use_unicode(true);
    f.render_widget(gauge, area);
}

fn render_playback_bar(f: &mut Frame, app: &App, area: Rect) {
    let title = app.playback_title.as_deref().unwrap_or("Unknown");
    
    let duration_str = app
        .playback_duration_str
        .as_deref()
        .unwrap_or("00:00/00:00");

    let status_str = if app.is_paused { " PAUSED " } else { " PLAYING " };
    let status_color = if app.is_paused { Color::Gray } else { THEME_ACCENT };

    // Calculate available space for title
    // status + space + duration + pipe + p + Arrows + x + Stop + padding
    // approx: 9 + 1 + 14 + 3 + 15 + 15 + 10 = 67
    let overhead = 70; 
    let available_width = area.width.saturating_sub(overhead) as usize;
    let displayed_title = if title.chars().count() > available_width && available_width > 3 {
        format!("{}...", title.chars().take(available_width.saturating_sub(3)).collect::<String>())
    } else {
        title.to_string()
    };

    let p = Paragraph::new(Line::from(vec![
        Span::styled(
            status_str,
            Style::default()
                .fg(Color::Black)
                .bg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("[{}] ", duration_str),
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            displayed_title,
            Style::default().fg(THEME_FG).add_modifier(Modifier::ITALIC),
        ),
        Span::raw(" | "),
        Span::styled(
            "p",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Pause | "),
        Span::styled(
            "Arrows",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Seek | "),
        Span::styled(
            "x",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Stop"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(THEME_ACCENT)),
    );
    f.render_widget(p, area);
}

fn render_format_selection(
    f: &mut Frame,
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
                // Try to extract height from "WIDTHxHEIGHT"
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
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">>");

    let mut state = TableState::default();
    state.select(selected_index);

    f.render_stateful_widget(table, area, &mut state);
}

fn render_action_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Select Action ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(THEME_BG).fg(THEME_FG))
        .border_style(Style::default().fg(THEME_HIGHLIGHT));

    let area = centered_rect(35, 40, area);
    f.render_widget(ratatui::widgets::Clear, area);

    let items: Vec<ListItem> = app
        .get_available_actions()
        .iter()
        .map(|action| {
            let key_str = match action.key {
                crossterm::event::KeyCode::Char(c) => c.to_string(),
                crossterm::event::KeyCode::Enter => "Enter".to_string(),
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
                .bg(THEME_ACCENT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">>");

    let mut state = ListState::default();
    // In a real app, you might want to manage the selected action index in App state
    // For now, we'll just show the list without a selection.
    // state.select(Some(0));

    f.render_stateful_widget(list, area, &mut state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
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

    // Show cursor if editing
    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((
            area.x + (app.cursor_position.saturating_sub(scroll)) as u16 + 1,
            area.y + 1,
        ));
    }
}

fn render_main_area(f: &mut Frame, app: &mut App, area: Rect, picker: &mut Picker) {
    if app.search_query.is_empty() {
        render_greeting_section(f, area);
        return;
    }

    let chunks = if app.show_downloads_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)]) // Results list takes full width
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area)
    };

    // Left: Results List
    let mut items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let is_selected = app.selected_playlist_indices.contains(&i);
            let checkbox = if is_selected { "[x] " } else { "[ ] " };

            let title = if v.video_type == crate::model::VideoType::Playlist {
                Span::styled(
                    format!(" {}{}. [PLAYLIST] {}", checkbox, i + 1, &v.title),
                    Style::default()
                        .fg(THEME_HIGHLIGHT)
                        .add_modifier(Modifier::BOLD),
                )
            } else if let Some(live_status) = &v.live_status {
                if live_status == "is_live" {
                    Span::styled(
                        format!(" {}{}. [LIVE NOW] {}", checkbox, i + 1, &v.title),
                        Style::default()
                            .fg(Color::Red) // Live streams are often red
                            .add_modifier(Modifier::BOLD),
                    )
                } else if live_status == "was_live" {
                    Span::styled(
                        format!(" {}{}. [WAS LIVE] {}", checkbox, i + 1, &v.title),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        format!(" {}{}. {}", checkbox, i + 1, &v.title),
                        Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
                    )
                }
            } else if v.parent_playlist_id.is_some() {
                Span::styled(
                    format!(" {}{}. [FROM PLAYLIST] {}", checkbox, i + 1, &v.title),
                    Style::default()
                        .fg(Color::Rgb(150, 220, 255))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!(" {}{}. {}", checkbox, i + 1, &v.title),
                    Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
                )
            };

            let mut second_line_spans = vec![ 
                Span::raw("      "), // Adjusted for checkbox width
                Span::styled(&v.channel, Style::default().fg(THEME_ACCENT)),
            ];

            if v.video_type == crate::model::VideoType::Playlist {
                if let Some(count) = v.playlist_count {
                    second_line_spans.push(Span::styled(
                        format!("  •  {} videos", count),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            } else if v.live_status.as_deref() == Some("is_live") {
                if let Some(viewers) = v.concurrent_view_count {
                    let viewers_fmt = if viewers > 1_000_000 {
                        format!("{:.1}M", viewers as f64 / 1_000_000.0)
                    } else if viewers > 1_000 {
                        format!("{:.1}K", viewers as f64 / 1_000.0)
                    } else {
                        viewers.to_string()
                    };
                    second_line_spans.push(Span::styled(
                        format!("  •  {} watching", viewers_fmt),
                        Style::default().fg(Color::Red),
                    ));
                } else {
                    second_line_spans.push(Span::styled(
                        "  •  LIVE",
                        Style::default().fg(Color::Red),
                    ));
                }
            } else {
                second_line_spans.push(Span::styled(
                    format!("  •  {}", v.duration_string),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let lines = vec![Line::from(title), Line::from(second_line_spans)];
            ListItem::new(lines).style(Style::default().fg(THEME_FG))
        })
        .collect();

    if !app.search_results.is_empty() && !app.is_url_mode {
        items.push(ListItem::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    " [ Load More Results... ] ",
                    Style::default()
                        .fg(THEME_HIGHLIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
        ]));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_BORDER))
                .title(if let Some((parent, _, _)) = app.playlist_stack.last() {
                    format!(" Playlist: {} ", parent.title) 
                } else {
                    " Results ".to_string()
                }),
        )
        .highlight_style(if app.state == AppState::Results {
            Style::default()
                .bg(THEME_HIGHLIGHT)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(Color::Rgb(40, 40, 50))
                .fg(Color::Gray)
        })
        .highlight_symbol(Span::styled(">>", Style::default().fg(if app.state == AppState::Results { THEME_HIGHLIGHT } else { Color::DarkGray })));

    let mut state = ListState::default();
    state.select(app.selected_result_index);

    f.render_stateful_widget(list, chunks[0], &mut state);

    // Right: Details/Preview
    if !app.show_downloads_panel { // Only render details if downloads panel is not shown
        let details_block = Block::default()
            .borders(Borders::ALL)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(THEME_BORDER))
            .title(" Details ");
        let inner_area = details_block.inner(chunks[1]);
        f.render_widget(details_block, chunks[1]);

        if let Some(idx) = app.selected_result_index {
            if idx < app.search_results.len() {
                if let Some(video) = app.search_results.get(idx) {
                    // Check for image
                    if let Some(img) = app.image_cache.get(&video.id) {
                        let original_img_width = img.width();
                        let original_img_height = img.height();

                        let available_width_for_image_cells = inner_area.width;

                        // Terminal cells are approx 2:1 (Height:Width). 
                        // We multiply by 0.5 to account for the fact that 1 row is as tall as 2 columns are wide.
                        let mut calculated_height = if original_img_width > 0 {
                            ((original_img_height as f64
                                / original_img_width as f64)
                                * available_width_for_image_cells as f64
                                * 0.5)
                                .round() as u16
                        } else {
                            0
                        };

                        // Limit the height so the image doesn't take over the whole screen on vertical monitors
                        calculated_height = calculated_height.clamp(2, 18);

                        let layout = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(calculated_height), // layout[0]
                                Constraint::Length(1),                 // layout[1] - THE BLANK LINE
                                Constraint::Min(0),                    // layout[2] - THE DETAILS
                            ])
                            .split(inner_area);

                        // If resize fails or protocol fails, we just don't render or it renders empty/block
                        let mut protocol = picker.new_resize_protocol(img.clone());
                        let image = ratatui_image::StatefulImage::new();
                        f.render_stateful_widget(image, layout[0], &mut protocol);

                        // Details Text
                        let details_area = layout[2];
                        if video.video_type == crate::model::VideoType::Playlist {
                            let text_lines = vec![
                                Line::from(vec![
                                    Span::styled(
                                        "Playlist: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(&video.title, Style::default().fg(THEME_FG)),
                                ]),
                                Line::from(vec![
                                    Span::styled(
                                        "Channel: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(&video.channel, Style::default().fg(THEME_FG)),
                                ]),
                                Line::from(vec![
                                    Span::styled(
                                        "Videos: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(
                                        video.playlist_count.unwrap_or(0).to_string(),
                                        Style::default().fg(THEME_FG),
                                    ),
                                ]),
                                Line::from(""), // Added spacing
                                Line::from(vec![Span::styled(
                                    " [ PLAYLIST ] ",
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(THEME_HIGHLIGHT)
                                        .add_modifier(Modifier::BOLD),
                                )])
                            ];
                            let p = Paragraph::new(text_lines).block(
                                Block::default()
                                    .borders(Borders::NONE)
                                    .padding(ratatui::widgets::Padding::left(1)),
                            );
                            f.render_widget(p, details_area);
                        } else {
                            let views = video.view_count.unwrap_or(0);
                            let views_fmt = if views > 1_000_000 {
                                format!("{:.1}M", views as f64 / 1_000_000.0)
                            } else if views > 1_000 {
                                format!("{:.1}K", views as f64 / 1_000.0)
                            } else {
                                views.to_string()
                            };

                            let mut text_lines = vec![
                                Line::from(vec![
                                    Span::styled(
                                        "Title: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(&video.title, Style::default().fg(THEME_FG)),
                                ]),
                                Line::from(vec![
                                    Span::styled(
                                        "Channel: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(&video.channel, Style::default().fg(THEME_FG)),
                                ]),
                            ];

                            // Only show Duration and Views if it's not a live stream (currently live)
                            if video.live_status.as_deref() == Some("is_live") {
                                if let Some(viewers) = video.concurrent_view_count {
                                    let viewers_fmt = if viewers > 1_000_000 {
                                        format!("{:.1}M", viewers as f64 / 1_000_000.0)
                                    } else if viewers > 1_000 {
                                        format!("{:.1}K", viewers as f64 / 1_000.0)
                                    } else {
                                        viewers.to_string()
                                    };
                                    text_lines.push(Line::from(vec![
                                        Span::styled(
                                            "Watching: ",
                                            Style::default()
                                                .fg(THEME_ACCENT)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled(viewers_fmt, Style::default().fg(Color::Red)),
                                    ]));
                                }
                            } else {
                                text_lines.push(Line::from(vec![
                                    Span::styled(
                                        "Duration: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(&video.duration_string, Style::default().fg(THEME_FG)),
                                ]));
                                text_lines.push(Line::from(vec![
                                    Span::styled(
                                        "Views: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(views_fmt, Style::default().fg(THEME_FG)),
                                ]));
                            }

                            if !video.is_partial {
                                let upload_date = format_upload_date(video.upload_date.as_deref());
                                text_lines.push(Line::from(vec![
                                    Span::styled(
                                        "Uploaded: ",
                                        Style::default()
                                            .fg(THEME_ACCENT)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(upload_date, Style::default().fg(THEME_FG)),
                                ]));
                                text_lines.push(Line::from("")); // Added spacing after Uploaded
                                // Show playlist info if available
                                if let Some(playlist_title) = &video.parent_playlist_title {
                                    text_lines.push(Line::from(vec![
                                        Span::styled(
                                            "From Playlist: ",
                                            Style::default()
                                                .fg(THEME_ACCENT)
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::styled(playlist_title, Style::default().fg(THEME_FG)),
                                    ]));
                                }
                            }

                            // Add Live Status Tag if available
                            if let Some(live_status_str) = &video.live_status {
                                let tag_text = match live_status_str.as_str() {
                                    "is_live" => Some(" [ LIVE NOW ] "),
                                    "was_live" => Some(" [ WAS LIVE ] "),
                                    _ => None, // Do not show "not_live" or other statuses
                                };

                                if let Some(text) = tag_text {
                                    text_lines.push(Line::from("")); // Spacing before the tag
                                    text_lines.push(Line::from(vec![Span::styled(
                                        text,
                                        Style::default()
                                            .fg(Color::Black)
                                            .bg(Color::Red) // Red background for live
                                            .add_modifier(Modifier::BOLD),
                                    )]));
                                }
                            }

                            let p = Paragraph::new(text_lines).block(
                                Block::default()
                                    .borders(Borders::NONE)
                                    .padding(ratatui::widgets::Padding::left(1)),
                            );
                            f.render_widget(p, details_area);
                        }
                    } else {
                        // No image yet
                        if video.video_type == crate::model::VideoType::Playlist {
                            let mut lines = vec![
                                format!("Playlist: {}", video.title),
                                format!("Channel: {}", video.channel),
                            ];
                            if let Some(count) = video.playlist_count {
                                lines.push(format!("Videos: {}", count));
                            }
                            lines.push(String::new());
                            lines.push("(Loading Thumbnail...)".to_string());
                            let p = Paragraph::new(lines.join("\n"));
                            f.render_widget(p, inner_area);
                        } else {
                            let views_str = if let Some(v) = video.view_count {
                                if v > 1_000_000 {
                                    format!("{:.1}M", v as f64 / 1_000_000.0)
                                } else if v > 1_000 {
                                    format!("{:.1}K", v as f64 / 1_000.0)
                                } else {
                                    v.to_string()
                                }
                            } else if video.is_partial {
                                "Loading...".to_string()
                            } else {
                                "N/A".to_string()
                            };

                            let upload_str = if video.is_partial {
                                String::new() // Don't show uploaded if partial
                            } else if video.upload_date.is_some() {
                                format_upload_date(video.upload_date.as_deref())
                            } else {
                                "Unknown".to_string()
                            };

                            let status_msg = if video.is_partial {
                                "(Fetching Details...)"
                            } else {
                                "(Loading Thumbnail...)"
                            };

                            let mut lines = vec![
                                format!("Title: {}", video.title),
                                format!("Channel: {}", video.channel),
                                format!("Duration: {}", video.duration_string),
                                format!("Views: {}", views_str),
                            ];

                            if !video.is_partial {
                                lines.push(format!("Uploaded: {}", upload_str));
                            }

                            lines.push(String::new());
                            lines.push(status_msg.to_string());

                            let p = Paragraph::new(lines.join("\n"));
                            f.render_widget(p, inner_area);
                        }
                    }
                } else if !app.is_url_mode {
                    // Load More selected
                    let text = "\n\n  Press ENTER to load 20 more results...";
                    let p = Paragraph::new(text).style(Style::default().fg(THEME_ACCENT));
                    f.render_widget(p, inner_area);
                }
                                            } else {
                                                let p = Paragraph::new("No video selected");
                                                f.render_widget(p, inner_area);
                                            }
                                        }        }

}

fn render_greeting_section(f: &mut Frame, area: Rect) {
    let area = centered_rect(60, 40, area);
    let text = vec![
        Line::from(vec![
            Span::styled(
                "R",
                Style::default()
                    .fg(THEME_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ataplay",
                Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search for videos or paste a URL.",
            Style::default().fg(THEME_ACCENT),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/",
                Style::default()
                    .fg(THEME_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to focus search bar.",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(THEME_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to browse and see actions.",
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];
    let p = Paragraph::new(text)
        .alignment(ratatui::layout::Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(THEME_BORDER)),
        );
    f.render_widget(p, area);
}

fn format_upload_date(raw: Option<&str>) -> String {
    if let Some(date) = raw {
        if date.len() == 8 {
            let y = &date[0..4];
            let m = &date[4..6];
            let d = &date[6..8];
            let months = [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ];
            if let Ok(m_idx) = m.parse::<usize>() {
                if m_idx >= 1 && m_idx <= 12 {
                    return format!("{} {}, {}", months[m_idx - 1], d, y);
                }
            }
            return format!("{}-{}-{}", y, m, d);
        }
        date.to_string()
    } else {
        "Unknown".to_string()
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Editing => "EDITING",
        InputMode::Loading => "LOADING",
    };

    let key_hints = match app.input_mode {
        InputMode::Normal => {
            match app.state {
                AppState::Downloads => "q: Quit | Tab: Back | d/b: Toggle | j/k: Nav | Space: Select | Enter: Options".to_string(),
                _ => {
                    let tab_hint = if app.show_downloads_panel { " | Tab: Downloads" } else { "" };
                    if !app.playlist_stack.is_empty() {
                        format!("q: Quit | /: Search{} | j/k: Nav | Space: Select | B: Back | Enter: Options", tab_hint)
                    } else {
                        format!("q: Quit | d/b: Toggle | /: Search{} | j/k: Nav | Enter: Open", tab_hint)
                    }
                }
            }
        }
        InputMode::Editing => "Esc: Normal Mode | Enter: Search".to_string(),
        InputMode::Loading => "Please wait...".to_string(),
    };

    let text = format!(" [{}] {} ", mode_str, key_hints);

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

fn render_downloads_view(f: &mut Frame, app: &mut App, area: Rect) {
    // Determine layout: if we have download tasks, split vertical. Otherwise full height for local.
    let has_downloads = !app.download_manager.task_order.is_empty();
    
    let chunks = if has_downloads {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40), // Active/Recent Downloads
                Constraint::Percentage(60), // Local Files
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    // Render Active Downloads (if any)
    if has_downloads {
        render_active_downloads(f, app, chunks[0]);
        render_local_files(f, app, chunks[1]);
    } else {
        render_local_files(f, app, chunks[0]);
    }
}

fn render_active_downloads(f: &mut Frame, app: &mut App, area: Rect) {
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
            .fg(THEME_ACCENT)
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
            let indicator = if is_focused { "> " } else { "  " };
            let checkbox = if app.selected_download_indices.contains(&i) { "[x] " } else { "[ ] " };

            let status_span = match &task.status {
                crate::model::download::DownloadStatus::Downloading => {
                    Span::styled("Downloading", Style::default().fg(Color::Green))
                }
                crate::model::download::DownloadStatus::Finished => {
                    Span::styled("Finished", Style::default().fg(Color::Cyan))
                }
                crate::model::download::DownloadStatus::Error(e) => {
                    Span::styled(format!("Error: {}", e), Style::default().fg(Color::Red))
                }
                crate::model::download::DownloadStatus::Paused => {
                    Span::styled("Paused", Style::default().fg(Color::Yellow))
                }
                crate::model::download::DownloadStatus::Canceled => {
                    Span::styled("Canceled", Style::default().fg(Color::DarkGray))
                }
                _ => Span::raw("Pending"),
            };

            let row_style = if is_focused {
                Style::default().bg(THEME_HIGHLIGHT).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{}{}{}", indicator, checkbox, task.title.clone())),
                Cell::from(task.total_size.clone()),
                Cell::from(create_progress_bar_string(
                    task.progress,
                    15,
                    THEME_ACCENT,
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
    
    let mut state = TableState::default();
    // We don't rely on table built-in highlight because we manage it per row above for better side-by-side control
    // but we can still select it for scrolling if needed. 
    // Actually, TableState is better for large lists. Let's keep it but use conditional style.
    state.select(if app.state == AppState::Downloads { app.selected_download_index } else { None });

    f.render_stateful_widget(table, area, &mut state);
}

fn render_local_files(f: &mut Frame, app: &mut App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("FILENAME"),
        Cell::from("SIZE"),
        Cell::from("FORMAT"),
        Cell::from("STATUS"),
    ])
    .style(
        Style::default()
            .fg(THEME_ACCENT)
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
            
            let indicator = if is_focused { "> " } else { "  " };
            let checkbox = if is_selected { "[x] " } else { "[ ] " };

            let status_span = if file.is_garbage {
                 Span::styled("Incomplete/Temp", Style::default().fg(Color::Yellow))
            } else {
                 Span::styled("Downloaded", Style::default().fg(Color::Green))
            };

            let row_style = if is_focused {
                Style::default().bg(THEME_HIGHLIGHT).fg(Color::Black).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!("{}{}{}", indicator, checkbox, file.name.clone())),
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

    let mut state = TableState::default();
    state.select(if app.state == AppState::Downloads { app.selected_local_file_index } else { None });

    f.render_stateful_widget(table, area, &mut state);
}
fn create_progress_bar_string(progress: f64, width: u16, fg_color: Color, bg_color: Color) -> Line<'static> {
    let bar_width = width.saturating_sub(6); // Account for percentage text and padding
    if bar_width == 0 {
        return Line::from(vec![Span::raw(format!("{:.1}%", progress))]);
    }

    let filled_chars = (bar_width as f64 * progress / 100.0).round() as u16;
    let empty_chars = bar_width.saturating_sub(filled_chars);

    let filled_part = Span::styled("█".repeat(filled_chars as usize), Style::default().fg(fg_color));
    let empty_part = Span::styled(" ".repeat(empty_chars as usize), Style::default().bg(bg_color));
    let percent_text = Span::styled(format!("{:.1}%", progress), Style::default().fg(THEME_FG));

    Line::from(vec![filled_part, empty_part, Span::raw(" "), percent_text])
}
