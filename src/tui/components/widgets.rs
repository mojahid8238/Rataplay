use ratatui::{
    prelude::Rect,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    style::{Color, Style},
};

use super::theme::THEME_FG;

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

pub fn get_width(s: &str) -> usize {
    s.chars().map(|c| {
        let cp = c as u32;
        if (0x1F300..=0x1F9FF).contains(&cp) || (0x2600..=0x26FF).contains(&cp) {
            2
        } else {
            1
        }
    }).sum()
}

pub fn truncate_str(s: &str, max_width: usize) -> String {
    if get_width(s) <= max_width {
        return s.to_string();
    }
    
    let mut result = String::new();
    let mut current_width = 0;
    
    for c in s.chars() {
        let w = get_width(&c.to_string());
        if current_width + w + 3 > max_width {
            result.push_str("...");
            break;
        }
        result.push(c);
        current_width += w;
    }
    result
}

pub fn create_progress_bar_string(progress: f64, width: u16, fg_color: Color, bg_color: Color) -> Line<'static> {
    let bar_width = width.saturating_sub(6); 
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