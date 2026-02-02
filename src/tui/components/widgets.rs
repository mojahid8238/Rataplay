use ratatui::{
    prelude::Rect,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    style::{Color, Style},
};

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

pub fn create_progress_bar_string(
    progress: f64, 
    width: u16, 
    fg_color: Color, 
    bg_color: Color,
    progress_style: &str
) -> Line<'static> {
    let bar_width = width as usize;
    if bar_width == 0 {
        return Line::from("");
    }

    let style_width = get_width(progress_style);
    let char_width = if style_width > 0 { style_width } else { 1 };
    let actual_style = if style_width > 0 { progress_style } else { "━" };

    let filled_width = (bar_width as f64 * progress / 100.0).round() as usize;
    let empty_width = bar_width.saturating_sub(filled_width);

    let filled_count = filled_width / char_width;
    let filled_rem = filled_width % char_width;
    
    let mut filled_str = actual_style.repeat(filled_count);
    if filled_rem > 0 {
        // Pad with spaces if the char doesn't fit perfectly
        filled_str.push_str(&" ".repeat(filled_rem));
    }

    // Use a subtle, fixed character for the empty/remaining part
    let empty_char = "━"; 
    let empty_str = empty_char.repeat(empty_width);

    let filled_part = Span::styled(filled_str, Style::default().fg(fg_color));
    let empty_part = Span::styled(empty_str, Style::default().fg(bg_color));

    Line::from(vec![filled_part, empty_part])
}