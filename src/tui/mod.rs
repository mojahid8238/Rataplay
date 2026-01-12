use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use ratatui_image::picker::Picker;
use crate::app::{App, InputMode, AppState};

pub fn ui(f: &mut Frame, app: &mut App, picker: &mut Picker) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // Search bar
                Constraint::Min(1),    // Main content
                Constraint::Length(3), // Status bar / Help
            ]
            .as_ref(),
        )
        .split(f.area());

    render_search_bar(f, app, chunks[0]);
    render_main_area(f, app, chunks[1], picker);
    render_status_bar(f, app, chunks[2]);

    if app.state == AppState::ActionMenu {
        render_action_menu(f, chunks[1]);
    }
    
    if app.state == AppState::FormatSelection {
        render_format_selection(f, app.selected_format_index, &app.formats, f.area());
    }
}

fn render_format_selection(f: &mut Frame, selected_index: Option<usize>, formats: &[crate::model::VideoFormat], area: Rect) {
    let block = Block::default()
        .title(" Select Format ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
        
    let area = centered_rect(80, 60, area);
    f.render_widget(ratatui::widgets::Clear, area);
    
    // Create list items
    let items: Vec<ListItem> = formats.iter().map(|fmt| {
        let text = format!("{: <10} | {: <5} | {: <12} | {: <20} | {}", 
            fmt.format_id, 
            fmt.ext, 
            fmt.resolution, 
            fmt.note, 
            fmt.filesize.map(|s| format!("~{}MB", s/1024/1024)).unwrap_or("?".to_string())
        );
        ListItem::new(text)
    }).collect();
    
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
        
    let mut state = ListState::default();
    state.select(selected_index);
    
    f.render_stateful_widget(list, area, &mut state);
}

fn render_action_menu(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Action Menu ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        
    let area = centered_rect(60, 20, area);
    f.render_widget(ratatui::widgets::Clear, area); // Clear underneath
    
    let text = vec![
        Line::from(vec![Span::raw("Select an action:")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::styled("[Enter/W] Watch Externally (mpv window)", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![Span::raw("   [T]    Watch in Terminal (tct/sixel)")]),
        Line::from(vec![Span::raw("   [A]    Listen (Audio Only)")]),
        Line::from(vec![Span::raw("   [D]    Download (Select Format)")]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![Span::raw("[Q/Esc]   Cancel")]),
    ];
    
    let p = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
        
    f.render_widget(p, area);
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
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
            InputMode::Loading => Style::default().fg(Color::Blue),
        })
        .block(Block::default().borders(Borders::ALL).title("Search / URL"));
    f.render_widget(input, area);

    // Show cursor if editing
    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((
            area.x + app.cursor_position as u16 + 1,
            area.y + 1,
        ));
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
        .block(Block::default().borders(Borders::ALL).title("Results"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(app.selected_result_index);
    
    f.render_stateful_widget(list, chunks[0], &mut state);

    // Right: Details/Preview
    let details_block = Block::default().borders(Borders::ALL).title("Details");
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
        InputMode::Loading => "Please wait...",
    };

    let text = format!("[{}] {}", mode_str, key_hints);
    let p = Paragraph::new(text)
        .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(p, area);
}
