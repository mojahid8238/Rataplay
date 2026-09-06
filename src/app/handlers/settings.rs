use crate::app::{App, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_date_filter_unit_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.date_filter_selection_index = if app.date_filter_selection_index == 0 {
                3
            } else {
                app.date_filter_selection_index - 1
            };
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.date_filter_selection_index = (app.date_filter_selection_index + 1) % 4;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.confirm_date_filter_unit(app.date_filter_selection_index);
        }
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Tab => {
            app.date_filter_selecting_unit = false;
            app.date_filter_selection_index = 3;
            app.status_message = Some("Filter Search: unchanged".to_string());
        }
        _ => {}
    }
}

pub fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.state = app.previous_app_state;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let current = app.settings_state.selected().unwrap_or(0);
            let next = if current > 0 {
                current - 1
            } else {
                crate::tui::components::settings::SettingItem::all().len() - 1
            };
            app.settings_state.select(Some(next));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let current = app.settings_state.selected().unwrap_or(0);
            let next = (current + 1) % crate::tui::components::settings::SettingItem::all().len();
            app.settings_state.select(Some(next));
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            if let Some(idx) = app.settings_state.selected() {
                let items = crate::tui::components::settings::SettingItem::all();
                if let Some(item) = items.get(idx) {
                    match item {
                        crate::tui::components::settings::SettingItem::Theme => {
                            app.theme_index = (app.theme_index + 1)
                                % crate::tui::components::theme::AVAILABLE_THEMES.len();
                            app.theme =
                                crate::tui::components::theme::AVAILABLE_THEMES[app.theme_index];
                            app.status_message = Some(format!("Theme: {}", app.theme.name));
                            app.save_config();
                            app.reload_config();
                        }
                        crate::tui::components::settings::SettingItem::Animation => {
                            app.toggle_animation();
                        }
                        crate::tui::components::settings::SettingItem::ShowLive => {
                            app.toggle_live();
                        }
                        crate::tui::components::settings::SettingItem::ShowPlaylists => {
                            app.toggle_playlists();
                        }
                        crate::tui::components::settings::SettingItem::SearchLimit => {
                            app.input_mode = InputMode::Editing;
                            app.settings_editing_item = Some(*item);
                            app.settings_input = app.search_limit.to_string();
                            app.settings_cursor_position = app.settings_input.len();
                            app.status_message = Some("Enter new Search Limit: ".to_string());
                        }
                        crate::tui::components::settings::SettingItem::PlaylistLimit => {
                            app.input_mode = InputMode::Editing;
                            app.settings_editing_item = Some(*item);
                            app.settings_input = app.playlist_limit.to_string();
                            app.settings_cursor_position = app.settings_input.len();
                            app.status_message = Some("Enter new Playlist Limit: ".to_string());
                        }
                        crate::tui::components::settings::SettingItem::DownloadDirectory => {
                            app.input_mode = InputMode::Editing;
                            app.settings_editing_item = Some(*item);
                            app.settings_input = app.download_directory.clone();
                            app.settings_cursor_position = app.settings_input.len();
                            app.status_message = Some("Enter new Download Directory: ".to_string());
                        }
                        crate::tui::components::settings::SettingItem::DateFilter => {
                            app.open_date_filter_selector();
                        }
                        crate::tui::components::settings::SettingItem::EnableLogging => {
                            app.settings.enable_logging = !app.settings.enable_logging;

                            let log_path = match crate::sys::config::Config::load() {
                                Ok(config) => config
                                    .get_log_path()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| "Unknown".to_string()),
                                Err(_) => "Unknown".to_string(),
                            };

                            app.status_message = Some(format!(
                                "Logging {}. Path: {}",
                                if app.settings.enable_logging {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                },
                                log_path
                            ));

                            app.save_config();
                            app.reload_config();
                        }
                        crate::tui::components::settings::SettingItem::UseCustomPaths => {
                            app.settings.use_custom_paths = !app.settings.use_custom_paths;
                            app.save_config();
                            app.reload_config();
                            app.status_message = Some(
                                "Toggled. Change binary paths in config.toml if needed."
                                    .to_string(),
                            );
                        }
                        crate::tui::components::settings::SettingItem::CookieMode => {
                            // Toggle between Off and Unsetted (enabled but not configured)
                            // User should configure source in config.toml
                            if app.settings.cookie_mode == crate::model::settings::CookieMode::Off {
                                app.settings.cookie_mode =
                                    crate::model::settings::CookieMode::Unsetted;
                                app.status_message = Some(
                                    "Enabled. Configure cookie source in config.toml.".to_string(),
                                );
                            } else {
                                app.settings.cookie_mode = crate::model::settings::CookieMode::Off;
                                app.status_message = Some("Disabled.".to_string());
                            }
                            app.save_config();
                            app.reload_config();
                        }
                        crate::tui::components::settings::SettingItem::ProgressStyle => {
                            app.input_mode = InputMode::Editing;
                            app.settings_editing_item = Some(*item);
                            app.settings_input = app.progress_style.clone();
                            app.settings_cursor_position = app.settings_input.len();
                            app.status_message = Some("Enter new Progress Style: ".to_string());
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn handle_settings_mouse(app: &mut App, x: u16, y: u16, double_click: bool) {
    if let Some(area) = app.settings_area {
        if super::is_in_rect(x, y, area) {
            let relative_y = y.saturating_sub(area.y).saturating_sub(1);
            let idx = app.settings_state.offset() + relative_y as usize;
            if idx < crate::tui::components::settings::SettingItem::all().len() {
                app.settings_state.select(Some(idx));
                if double_click {
                    super::handle_key_event(
                        app,
                        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
                    );
                }
            }
        } else {
            app.state = app.previous_app_state;
        }
    }
}
