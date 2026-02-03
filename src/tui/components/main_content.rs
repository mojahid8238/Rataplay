use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use ratatui_image::picker::Picker;

use crate::app::{App, AppState};
use crate::model::VideoType;

use super::logo::Logo;
use super::widgets::{centered_rect, truncate_str};

pub fn render_main_area(f: &mut ratatui::Frame, app: &mut App, area: Rect, picker: &mut Picker) {
    if app.search_query.is_empty() {
        render_greeting_section(f, app, area);
        return;
    }

    let chunks = if app.show_downloads_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area)
    };

    let list_inner_width = chunks[0].width.saturating_sub(6) as usize;

    let mut items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let is_selected = app.selected_playlist_indices.contains(&i);
            let checkbox = if is_selected { "[x] " } else { "[ ] " };
            let index_prefix = format!(" {}{}. ", checkbox, i + 1);
            let prefix_len = index_prefix.chars().count();

            let title_line = if v.video_type == VideoType::Playlist {
                let tag = "[PLAYLIST] ";
                let avail = list_inner_width.saturating_sub(prefix_len + tag.len());
                let display_title = truncate_str(&v.title, avail);
                Line::from(vec![
                    Span::styled(
                        index_prefix,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        tag,
                        Style::default()
                            .fg(app.theme.highlight)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        display_title,
                        Style::default()
                            .fg(app.theme.highlight)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if let Some(live_status) = &v.live_status {
                let tag = if live_status == "is_live" {
                    "[LIVE NOW] "
                } else if live_status == "was_live" {
                    "[WAS LIVE] "
                } else {
                    ""
                };
                let tag_color = if live_status == "is_live" {
                    Color::Red
                } else {
                    Color::DarkGray
                };

                let avail = list_inner_width.saturating_sub(prefix_len + tag.len());
                let display_title = truncate_str(&v.title, avail);

                Line::from(vec![
                    Span::styled(
                        index_prefix,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    if !tag.is_empty() {
                        Span::styled(
                            tag,
                            Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("")
                    },
                    Span::styled(
                        display_title,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if v.parent_playlist_id.is_some() {
                let tag = "[FROM PLAYLIST] ";
                let avail = list_inner_width.saturating_sub(prefix_len + tag.len());
                let display_title = truncate_str(&v.title, avail);

                Line::from(vec![
                    Span::styled(
                        index_prefix,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        tag,
                        Style::default()
                            .fg(app.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        display_title,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                let avail = list_inner_width.saturating_sub(prefix_len);
                let display_title = truncate_str(&v.title, avail);

                Line::from(vec![
                    Span::styled(
                        index_prefix,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        display_title,
                        Style::default()
                            .fg(app.theme.fg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            };

            let channel_avail = list_inner_width.saturating_sub(6 + 15);
            let display_channel = truncate_str(&v.channel, channel_avail);

            let mut second_line_spans = vec![
                Span::raw("      "),
                Span::styled(display_channel, Style::default().fg(app.theme.accent)),
            ];

            if v.video_type == VideoType::Playlist {
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
                    second_line_spans
                        .push(Span::styled("  •  LIVE", Style::default().fg(Color::Red)));
                }
            } else {
                second_line_spans.push(Span::styled(
                    format!("  •  {}", v.duration_string),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            let lines = vec![title_line, Line::from(second_line_spans)];
            ListItem::new(lines).style(Style::default().fg(app.theme.fg))
        })
        .collect();

    if !app.search_results.is_empty() && (!app.is_url_mode || app.is_playlist_mode) {
        items.push(ListItem::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    " [ Load More Results... ] ",
                    Style::default()
                        .fg(app.theme.highlight)
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
                .border_style(Style::default().fg(app.theme.border))
                .title(if let Some((parent, _, _)) = app.playlist_stack.last() {
                    format!(" Playlist: {} ", parent.title)
                } else {
                    " Results ".to_string()
                }),
        )
        .highlight_style(if app.state == AppState::Results {
            Style::default()
                .bg(app.theme.highlight)
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(app.theme.bg).fg(Color::Gray)
        })
        .highlight_symbol(Span::styled(
            "┃ ",
            Style::default().fg(if app.state == AppState::Results {
                app.theme.highlight
            } else {
                Color::DarkGray
            }),
        ));

    app.main_list_state.select(app.selected_result_index);
    f.render_stateful_widget(list, chunks[0], &mut app.main_list_state);

    if !app.show_downloads_panel {
        let details_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(app.theme.border))
            .title(" Details ");
        let inner_area = details_block.inner(chunks[1]);
        f.render_widget(details_block, chunks[1]);

        if let Some(idx) = app.selected_result_index {
            if let Some(video) = app.search_results.get(idx) {
                if let Some(img) = app.image_cache.get(&video.id) {
                    let original_img_width = img.width();
                    let original_img_height = img.height();

                    let available_width_for_image_cells = inner_area.width;

                    let mut calculated_height = if original_img_width > 0 {
                        ((original_img_height as f64 / original_img_width as f64)
                            * available_width_for_image_cells as f64
                            * 0.5)
                            .round() as u16
                    } else {
                        0
                    };

                    calculated_height = calculated_height.clamp(2, 18);

                    let layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(calculated_height),
                            Constraint::Length(1),
                            Constraint::Min(0),
                        ])
                        .split(inner_area);

                    let mut protocol = picker.new_resize_protocol(img.clone());
                    let image = ratatui_image::StatefulImage::new();
                    f.render_stateful_widget(image, layout[0], &mut protocol);

                    let details_area = layout[2];
                    if video.video_type == VideoType::Playlist {
                        let text_lines = vec![
                            Line::from(vec![
                                Span::styled(
                                    "Playlist: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&video.title, Style::default().fg(app.theme.fg)),
                            ]),
                            Line::from(vec![
                                Span::styled(
                                    "Channel: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&video.channel, Style::default().fg(app.theme.fg)),
                            ]),
                            Line::from(vec![
                                Span::styled(
                                    "Videos: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    video.playlist_count.unwrap_or(0).to_string(),
                                    Style::default().fg(app.theme.fg),
                                ),
                            ]),
                            Line::from(""),
                            Line::from(vec![Span::styled(
                                " [ PLAYLIST ] ",
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(app.theme.highlight)
                                    .add_modifier(Modifier::BOLD),
                            )]),
                        ];
                        let p = Paragraph::new(text_lines).wrap(Wrap { trim: true }).block(
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
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&video.title, Style::default().fg(app.theme.fg)),
                            ]),
                            Line::from(vec![
                                Span::styled(
                                    "Channel: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&video.channel, Style::default().fg(app.theme.fg)),
                            ]),
                        ];

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
                                            .fg(app.theme.accent)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(viewers_fmt, Style::default().fg(Color::Red)),
                                ]));
                            } else {
                                text_lines.push(Line::from(vec![Span::styled(
                                    "  •  LIVE",
                                    Style::default().fg(Color::Red),
                                )]));
                            }
                        } else {
                            text_lines.push(Line::from(vec![
                                Span::styled(
                                    "Duration: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    &video.duration_string,
                                    Style::default().fg(app.theme.fg),
                                ),
                            ]));
                            text_lines.push(Line::from(vec![
                                Span::styled(
                                    "Views: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(views_fmt, Style::default().fg(app.theme.fg)),
                            ]));
                        }

                        if !video.is_partial {
                            let upload_date = format_upload_date(video.upload_date.as_deref());
                            text_lines.push(Line::from(vec![
                                Span::styled(
                                    "Uploaded: ",
                                    Style::default()
                                        .fg(app.theme.accent)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(upload_date, Style::default().fg(app.theme.fg)),
                            ]));
                            text_lines.push(Line::from(""));
                            if let Some(playlist_title) = &video.parent_playlist_title {
                                text_lines.push(Line::from(vec![
                                    Span::styled(
                                        "From Playlist: ",
                                        Style::default()
                                            .fg(app.theme.accent)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                    Span::styled(playlist_title, Style::default().fg(app.theme.fg)),
                                ]));
                            }
                        }

                        if let Some(live_status_str) = &video.live_status {
                            let tag_text = match live_status_str.as_str() {
                                "is_live" => Some(" [ LIVE NOW ] "),
                                "was_live" => Some(" [ WAS LIVE ] "),
                                _ => None,
                            };

                            if let Some(text) = tag_text {
                                text_lines.push(Line::from(""));
                                text_lines.push(Line::from(vec![Span::styled(
                                    text,
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Red)
                                        .add_modifier(Modifier::BOLD),
                                )]));
                            }
                        }

                        let p = Paragraph::new(text_lines).wrap(Wrap { trim: true }).block(
                            Block::default()
                                .borders(Borders::NONE)
                                .padding(ratatui::widgets::Padding::left(1)),
                        );
                        f.render_widget(p, details_area);
                    }
                } else {
                    if video.video_type == VideoType::Playlist {
                        let mut lines = vec![
                            format!("Playlist: {}", video.title),
                            format!("Channel: {}", video.channel),
                        ];
                        if let Some(count) = video.playlist_count {
                            lines.push(format!("Videos: {}", count));
                        }
                        lines.push(String::new());
                        lines.push("(Loading Thumbnail...)".to_string());
                        let p = Paragraph::new(lines.join("\n")).wrap(Wrap { trim: true });
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
                            String::new()
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

                        let p = Paragraph::new(lines.join("\n")).wrap(Wrap { trim: true });
                        f.render_widget(p, inner_area);
                    }
                }
            } else if (!app.is_url_mode || app.is_playlist_mode) && idx == app.search_results.len()
            {
                let text = "\n\n  Press ENTER to load more results...";
                let p = Paragraph::new(text).style(Style::default().fg(app.theme.accent));
                f.render_widget(p, inner_area);
            } else {
                let p = Paragraph::new("No video selected");
                f.render_widget(p, inner_area);
            }
        }
    }
}

pub fn render_greeting_section(f: &mut ratatui::Frame, app: &App, area: Rect) {
    let area = centered_rect(60, 40, area);

    // Main block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.border));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Pet area (Logo + Bubble)
            Constraint::Min(0),     // Instructions
        ])
        .split(inner_area);

    // 1. Render Pet (The Talking Animated Banner)
    let pet = Logo::new(app.pet_frame, app.theme, app.animation_mode);
    f.render_widget(pet, chunks[0]);

    // 2. Render Instructions
    let instructions = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search for videos or paste a URL to start.",
            Style::default().fg(app.theme.accent),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [ / ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Focus Search", Style::default().fg(Color::DarkGray)),
            Span::raw("    "),
            Span::styled(
                " [ Enter ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Select / Actions", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                " [ d ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Downloads", Style::default().fg(Color::DarkGray)),
            Span::raw("       "),
            Span::styled(
                " [ Ctrl+t ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Cycle Themes", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                " [ Ctrl+a ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Toggle Animations", Style::default().fg(Color::DarkGray)),
            Span::raw("    "),
            Span::styled(
                " [ q ] ",
                Style::default()
                    .fg(app.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let p = Paragraph::new(instructions).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(p, chunks[1]);
}

pub fn format_upload_date(raw: Option<&str>) -> String {
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
