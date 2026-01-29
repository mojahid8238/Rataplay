use ratatui::{
    prelude::*,
    widgets::*,
};
use crate::app::App;
use super::widgets::centered_rect_fixed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingItem {
    Theme,
    Animation,
    SearchLimit,
    PlaylistLimit,
    DownloadDirectory,
    ShowLive,
    ShowPlaylists,
    EnableLogging,
    UseCustomPaths,
    CookieMode,
}

impl SettingItem {
    pub fn all() -> &'static [Self] {
        &[
            Self::Theme,
            Self::Animation,
            Self::SearchLimit,
            Self::PlaylistLimit,
            Self::DownloadDirectory,
            Self::ShowLive,
            Self::ShowPlaylists,
            Self::EnableLogging,
            Self::UseCustomPaths,
            Self::CookieMode,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Theme => "Theme",
            Self::Animation => "Logo Animation",
            Self::SearchLimit => "Search Results Limit",
            Self::PlaylistLimit => "Playlist Items Limit",
            Self::DownloadDirectory => "Download Directory",
            Self::ShowLive => "Show Live Streams",
            Self::ShowPlaylists => "Show Playlists",
            Self::EnableLogging => "Enable Logging",
            Self::UseCustomPaths => "Use Custom Paths",
            Self::CookieMode => "Cookie Mode",
        }
    }
}

pub fn render_settings_menu(f: &mut Frame, app: &mut App, area: Rect) {
    let items = SettingItem::all();
    
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.theme.bg).fg(app.theme.fg))
        .border_style(Style::default().fg(app.theme.highlight));

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|item| {
            let value = match item {
                SettingItem::Theme => app.theme.name.to_string(),
                SettingItem::Animation => app.animation_mode.name().to_string(),
                SettingItem::SearchLimit => app.search_limit.to_string(),
                SettingItem::PlaylistLimit => app.playlist_limit.to_string(),
                SettingItem::DownloadDirectory => app.download_directory.clone(),
                SettingItem::ShowLive => (if app.show_live { "On" } else { "Off" }).to_string(),
                SettingItem::ShowPlaylists => (if app.show_playlists { "On" } else { "Off" }).to_string(),
                SettingItem::EnableLogging => (if app.settings.enable_logging { "On" } else { "Off" }).to_string(),
                SettingItem::UseCustomPaths => (if app.settings.use_custom_paths { "On" } else { "Off" }).to_string(),
                SettingItem::CookieMode => match &app.settings.cookie_mode {
                    crate::model::settings::CookieMode::Off => "Off".to_string(),
                    crate::model::settings::CookieMode::File(_) => "File (Configured)".to_string(),
                    crate::model::settings::CookieMode::Browser(b) => format!("Browser ({})", b),
                },
            };

            let content = Line::from(vec![
                Span::styled(format!("{:<22}: ", item.name()), Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(value, Style::default().fg(app.theme.accent)),
            ]);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(list_items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(app.theme.highlight)
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("┃ ");

    let area = centered_rect_fixed(60, (items.len() + 2) as u16, area);
    app.settings_area = Some(area);
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut app.settings_state);

    if let Some(item) = app.settings_editing_item {
        render_input_popup(f, app, item);
    }
}

fn render_input_popup(f: &mut Frame, app: &App, item: SettingItem) {
    let area = centered_rect_fixed(50, 3, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!(" Edit {} ", item.name()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent));

    let width = (area.width as usize).saturating_sub(2);
    let scroll = app.settings_cursor_position.saturating_sub(width.saturating_sub(1));
    let display_text: String = app.settings_input.chars().skip(scroll).take(width).collect();

    let input = Paragraph::new(display_text)
        .style(Style::default().fg(app.theme.accent).add_modifier(Modifier::BOLD))
        .block(block);

    f.render_widget(input, area);
    f.set_cursor_position((
        area.x + (app.settings_cursor_position.saturating_sub(scroll)) as u16 + 1,
        area.y + 1,
    ));
}
