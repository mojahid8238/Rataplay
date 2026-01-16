use crate::app::{App, AppState, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
    Frame,
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

    if app.download_progress.is_some() {
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
    render_main_area(f, app, main_layout[1], picker);

    let mut current_idx = 2;
    if app.playback_title.is_some() {
        render_playback_bar(f, app, main_layout[current_idx]);
        current_idx += 1;
    }

    if let Some(progress) = app.download_progress {
        render_download_gauge(
            f,
            progress,
            app.download_status.as_deref().unwrap_or("Downloading..."),
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
        .label(Span::styled(
            format!(" {} {:.0}% ", status, progress * 100.0),
            Style::default().fg(THEME_FG).add_modifier(Modifier::BOLD),
        ))
        .ratio(progress.into())
        .use_unicode(true);
    f.render_widget(gauge, area);
}

fn render_playback_bar(f: &mut Frame, app: &App, area: Rect) {
    let title = app.playback_title.as_deref().unwrap_or("Unknown");
    let is_paused = app.is_paused;
    let duration_str = app
        .playback_duration_str
        .as_deref()
        .unwrap_or("00:00/00:00");

    let status_str = if app.is_finishing {
        " FINISHED "
    } else if is_paused {
        " PAUSED "
    } else {
        " PLAYING "
    };
    let status_color = if app.is_finishing {
        Color::LightRed
    } else if is_paused {
        THEME_HIGHLIGHT
    } else {
        THEME_ACCENT
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
            title,
            Style::default().fg(THEME_FG).add_modifier(Modifier::ITALIC),
        ),
        Span::raw(" | "),
        Span::styled(
            "p",
            Style::default()
                .fg(THEME_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": Play/Pause | "),
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
    .highlight_symbol(">> ");

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

    let area = centered_rect(35, 20, area);
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

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
        .split(area);

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
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 40, 50))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(Span::styled(">> ", Style::default().fg(THEME_HIGHLIGHT)));

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
        if idx < app.search_results.len() {
            if let Some(video) = app.search_results.get(idx) {
                // Check for image
                if let Some(img) = app.image_cache.get(&video.id) {
                    // Render image
                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(inner_area);

                    // If resize fails or protocol fails, we just don't render or it renders empty/block
                    let mut protocol = picker.new_resize_protocol(img.clone());
                    let image = ratatui_image::StatefulImage::new(None);
                    f.render_stateful_widget(image, layout[0], &mut protocol);

                    // Details Text
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
                            Line::from(""),
                            Line::from(vec![Span::styled(
                                " [ TYPE: PLAYLIST ] ",
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(THEME_HIGHLIGHT)
                                    .add_modifier(Modifier::BOLD),
                            )]),
                        ];
                        let p = Paragraph::new(text_lines).block(
                            Block::default()
                                .borders(Borders::NONE)
                                .padding(ratatui::widgets::Padding::left(1)),
                        );
                        f.render_widget(p, layout[1]);
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
                            Line::from(vec![
                                Span::styled(
                                    "Duration: ",
                                    Style::default()
                                        .fg(THEME_ACCENT)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&video.duration_string, Style::default().fg(THEME_FG)),
                            ]),
                            Line::from(vec![
                                Span::styled(
                                    "Views: ",
                                    Style::default()
                                        .fg(THEME_ACCENT)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(views_fmt, Style::default().fg(THEME_FG)),
                            ]),
                        ];

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

                            // Show playlist info if available
                            if let Some(playlist_title) = &video.parent_playlist_title {
                                text_lines.push(Line::from(""));
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

                        let p = Paragraph::new(text_lines).block(
                            Block::default()
                                .borders(Borders::NONE)
                                .padding(ratatui::widgets::Padding::left(1)),
                        );
                        f.render_widget(p, layout[1]);
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
}

fn render_greeting_section(f: &mut Frame, area: Rect) {
    let area = centered_rect(60, 40, area);
    let text = vec![
        Line::from(vec![
            Span::styled(
                "V",
                Style::default()
                    .fg(THEME_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ivid",
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
            if !app.playlist_stack.is_empty() {
                "q: Quit | /: Search | j/k: Nav | Space: Select | B: Back | Enter: Options"
            } else {
                "q: Quit | /: Search | j/k: Nav | Enter: Open"
            }
        }
        InputMode::Editing => "Esc: Normal Mode | Enter: Search",
        InputMode::Loading => "Please wait...",
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
