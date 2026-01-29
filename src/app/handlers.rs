use super::{
    App, AppAction, AppState, DownloadControl, InputMode
};
use crate::model::Video;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use std::time::{Duration, Instant};
use crate::sys::local;
use super::actions;
use super::updates;

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;
            let double_click = is_double_click(app, x, y);

            // Handle Overlays first
            if app.state == AppState::ActionMenu {
                if let Some(area) = app.action_menu_area {
                    if is_in_rect(x, y, area) {
                        // Click inside action menu
                        let relative_y = y.saturating_sub(area.y).saturating_sub(1); // -1 for border
                        let idx = app.action_menu_state.offset() + relative_y as usize;
                        let actions = actions::get_available_actions(app);
                        if let Some(action) = actions.get(idx) {
                             // Execute action
                             let code = action.key;
                             handle_key_event(app, KeyEvent::new(code, KeyModifiers::empty()));
                        }
                    } else {
                         // Click outside -> dismiss
                        app.state = app.previous_app_state;
                    }
                }
                return;
            }

            if app.state == AppState::FormatSelection {
                if let Some(area) = app.format_selection_area {
                    if is_in_rect(x, y, area) {
                        let header_height = 2; // Header + Margin
                        let list_start_y = area.y + 1 + header_height; // Border + Header
                        if y >= list_start_y {
                            let relative_y = y - list_start_y;
                            let idx = app.format_selection_state.offset() + relative_y as usize;
                            if idx < app.formats.len() {
                                app.selected_format_index = Some(idx);
                                if double_click {
                                    handle_key_event(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                                }
                            }
                        }
                    } else {
                        // Click outside -> back to action menu?
                        // Esc does this, so let's match it.
                        app.state = AppState::ActionMenu;
                    }
                }
                return;
            }

            if app.state == AppState::Settings {
                if let Some(area) = app.settings_area {
                    if is_in_rect(x, y, area) {
                        let relative_y = y.saturating_sub(area.y).saturating_sub(1);
                        let idx = app.settings_state.offset() + relative_y as usize;
                        if idx < crate::tui::components::settings::SettingItem::all().len() {
                            app.settings_state.select(Some(idx));
                            if double_click {
                                handle_key_event(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
                            }
                        }
                    } else {
                        app.state = app.previous_app_state;
                    }
                }
                return;
            }

            // Playback Bar
            if let Some(area) = app.playback_bar_area {
                if is_in_rect(x, y, area) {
                    actions::toggle_pause(app);
                    return;
                }
            }

            // Search Bar
            if is_in_rect(x, y, app.search_bar_area) {
                app.input_mode = InputMode::Editing;
                return;
            }

            // Downloads Panel
            if app.show_downloads_panel {
                if let Some(area) = app.downloads_area {
                    if is_in_rect(x, y, area) {
                        if app.state != AppState::Downloads {
                             app.previous_app_state = app.state;
                             app.state = AppState::Downloads;
                             actions::refresh_local_files(app);
                        }
                        
                        // Hit testing for Downloads
                        let has_active = !app.download_manager.task_order.is_empty();
                        let active_height = if has_active {
                             (area.height as f64 * 0.4).round() as u16
                        } else {
                             0
                        };
                        
                        if has_active && y < area.y + active_height {
                            // Active downloads section
                             let list_start_y = area.y + 1 + 1; // Border + Header
                             if y >= list_start_y {
                                 let relative_y = y - list_start_y;
                                 let idx = app.downloads_active_state.offset() + relative_y as usize;
                                 if idx < app.download_manager.task_order.len() {
                                     app.selected_download_index = Some(idx);
                                     app.selected_local_file_index = None;
                                     if double_click {
                                         app.previous_app_state = app.state;
                                         app.state = AppState::ActionMenu;
                                     }
                                 }
                             }
                        } else {
                            // Local files section
                            let local_start_y = if has_active {
                                area.y + active_height + 1 + 1 // + Border + Header of local block
                            } else {
                                area.y + 1 + 1 // Border + Header
                            };
                            
                            if y >= local_start_y {
                                let relative_y = y - local_start_y;
                                let idx = app.downloads_local_state.offset() + relative_y as usize;
                                if idx < app.local_files.len() {
                                    app.selected_local_file_index = Some(idx);
                                    app.selected_download_index = None;
                                    if double_click {
                                        app.previous_app_state = app.state;
                                        app.state = AppState::ActionMenu;
                                    }
                                }
                            }
                        }
                        return;
                    }
                }
            }

            // Main Content
            if is_in_rect(x, y, app.main_content_area) {
                if app.state == AppState::Downloads {
                    app.state = AppState::Results;
                }
                
                // Hit testing for Main List
                // Items are 2 lines tall.
                let list_start_y = app.main_content_area.y + 1; // Border
                if y >= list_start_y {
                    let relative_y = y - list_start_y;
                    let item_index = app.main_list_state.offset() + (relative_y / 2) as usize;
                    
                    if item_index < app.search_results.len() {
                        app.selected_result_index = Some(item_index);
                        if double_click {
                            app.previous_app_state = app.state;
                            app.state = AppState::ActionMenu;
                        }
                    } else if !app.is_url_mode && item_index == app.search_results.len() {
                         // Clicked "Load More" (approx)
                         actions::load_more(app);
                    }
                }
                return;
            }
        }
        MouseEventKind::ScrollUp => {
             match app.state {
                 AppState::Results => updates::move_selection(app, -1),
                 AppState::Downloads => {
                     if let Some(idx) = app.selected_local_file_index {
                        if idx > 0 {
                            app.selected_local_file_index = Some(idx - 1);
                        } else if !app.download_manager.task_order.is_empty() {
                            app.selected_local_file_index = None;
                            app.selected_download_index = Some(app.download_manager.task_order.len() - 1);
                        }
                    } else if let Some(idx) = app.selected_download_index {
                        if idx > 0 {
                            app.selected_download_index = Some(idx - 1);
                        }
                    } else {
                        if !app.local_files.is_empty() {
                            app.selected_local_file_index = Some(0);
                        } else if !app.download_manager.task_order.is_empty() {
                            app.selected_download_index = Some(0);
                        }
                    }
                 }
                 AppState::FormatSelection => {
                    if let Some(idx) = app.selected_format_index {
                        if idx > 0 {
                            app.selected_format_index = Some(idx - 1);
                        }
                    }
                 }
                 _ => {}
             }
        }
        MouseEventKind::ScrollDown => {
             match app.state {
                 AppState::Results => updates::move_selection(app, 1),
                 AppState::Downloads => {
                     if let Some(idx) = app.selected_download_index {
                        if idx < app.download_manager.task_order.len() - 1 {
                            app.selected_download_index = Some(idx + 1);
                        } else if !app.local_files.is_empty() {
                            app.selected_download_index = None;
                            app.selected_local_file_index = Some(0);
                        }
                    } else if let Some(idx) = app.selected_local_file_index {
                        if idx < app.local_files.len().saturating_sub(1) {
                            app.selected_local_file_index = Some(idx + 1);
                        }
                    } else {
                        if !app.download_manager.task_order.is_empty() {
                            app.selected_download_index = Some(0);
                        } else if !app.local_files.is_empty() {
                            app.selected_local_file_index = Some(0);
                        }
                    }
                 }
                 AppState::FormatSelection => {
                    if let Some(idx) = app.selected_format_index {
                        if idx < app.formats.len().saturating_sub(1) {
                            app.selected_format_index = Some(idx + 1);
                        }
                    }
                 }
                 _ => {}
             }
        }
        _ => {}
    }
}

fn is_double_click(app: &mut App, x: u16, y: u16) -> bool {
    let now = Instant::now();
    let threshold = Duration::from_millis(500);
    
    if let (Some(last_time), Some(last_pos)) = (app.last_click_time, app.last_click_pos) {
        if now.duration_since(last_time) < threshold && last_pos == (x, y) {
            app.last_click_time = None; 
            return true;
        }
    }
    app.last_click_time = Some(now);
    app.last_click_pos = Some((x, y));
    false
}

fn is_in_rect(x: u16, y: u16, area: ratatui::layout::Rect) -> bool {
    x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
}

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
    let code = match app.input_mode {
        InputMode::Editing => key.code,
        _ => match key.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_lowercase().next().unwrap_or(c)),
            _ => key.code,
        },
    };

    log::debug!("Key event: {:?}, input_mode: {:?}", code, app.input_mode);

    match app.input_mode {
        InputMode::Normal => {
            if code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.change_theme();
                return;
            }

            if code == KeyCode::Char('a') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_animation();
                return;
            }

            if code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_live();
                return;
            }

            if code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.toggle_playlists();
                return;
            }

            if code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.state == AppState::Settings {
                    app.state = app.previous_app_state;
                } else {
                    app.previous_app_state = app.state;
                    app.state = AppState::Settings;
                    app.settings_state.select(Some(0));
                }
                return;
            }


            match app.state {
                AppState::Settings => match code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.state = app.previous_app_state;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let current = app.settings_state.selected().unwrap_or(0);
                        let next = if current > 0 { current - 1 } else { crate::tui::components::settings::SettingItem::all().len() - 1 };
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
                                        app.theme_index = (app.theme_index + 1) % crate::tui::components::theme::AVAILABLE_THEMES.len();
                                        app.theme = crate::tui::components::theme::AVAILABLE_THEMES[app.theme_index];
                                        app.status_message = Some(format!("Theme: {}", app.theme.name));
                                        app.save_config();
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
                                    crate::tui::components::settings::SettingItem::EnableLogging => {
                                        app.settings.enable_logging = !app.settings.enable_logging;
                                        app.save_config(); 
                                    }
                                    crate::tui::components::settings::SettingItem::UseCustomPaths => {
                                        app.settings.use_custom_paths = !app.settings.use_custom_paths;
                                        app.save_config();
                                        app.status_message = Some("Restart required for path changes to take effect.".to_string());
                                    }
                                    crate::tui::components::settings::SettingItem::CookieMode => {
                                        // Cycle: Off -> File (if available) -> Browser (if available) -> Off
                                        // Simplification: Toggle Off <-> Configured Default
                                        // Since we don't have a UI to input paths/browser names yet, best we can do is toggle between Off and whatever was in the config/struct initially.
                                        // Actually, let's try a cycle if we can restore previous state, but we don't store "previous state".
                                        
                                        // Better approach: 
                                        // If Off -> Switch to File (if path exists in struct) or Browser (if name exists in struct) or Default
                                        // But wait, the struct holds the *current* state.
                                        
                                        match app.settings.cookie_mode {
                                            crate::model::settings::CookieMode::Off => {
                                                // Try to switch to File if we have one configured (or default one?)
                                                if let Some(path) = &app.settings.cookie_file {
                                                     app.settings.cookie_mode = crate::model::settings::CookieMode::File(path.clone());
                                                } else if let Some(browser) = &app.settings.browser_name {
                                                     app.settings.cookie_mode = crate::model::settings::CookieMode::Browser(browser.clone());
                                                } else {
                                                     // Fallback or nothing? maybe just show status
                                                     app.status_message = Some("No cookie source configured in config.toml".to_string());
                                                }
                                            }
                                            crate::model::settings::CookieMode::File(_) => {
                                                if let Some(browser) = &app.settings.browser_name {
                                                     app.settings.cookie_mode = crate::model::settings::CookieMode::Browser(browser.clone());
                                                } else {
                                                     app.settings.cookie_mode = crate::model::settings::CookieMode::Off;
                                                }
                                            }
                                            crate::model::settings::CookieMode::Browser(_) => {
                                                app.settings.cookie_mode = crate::model::settings::CookieMode::Off;
                                            }
                                        }
                                        app.save_config();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                },
                AppState::FormatSelection => match code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.state = AppState::ActionMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(idx) = app.selected_format_index {
                            if idx > 0 {
                                app.selected_format_index = Some(idx - 1);
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(idx) = app.selected_format_index {
                            if idx < app.formats.len().saturating_sub(1) {
                                app.selected_format_index = Some(idx + 1);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = app.selected_format_index {
                            if let Some(fmt) = app.formats.get(idx) {
                                if let Some(res_idx) = app.selected_result_index {
                                    if let Some(video) = app.search_results.get(res_idx) {
                                        // Add to manager and start download
                                        app.download_manager.add_task(video, &fmt.format_id);
                                        let _ = app
                                            .new_download_tx
                                            .send((video.clone(), fmt.format_id.clone()));
                                        app.state = AppState::Results;
                                        app.status_message =
                                            Some("Download started...".to_string());
                                        return;
                                    }
                                }
                            }
                        }
                        app.state = AppState::Results; 
                    }
                    _ => {}
                },
                AppState::Downloads => match code {
                    KeyCode::Tab | KeyCode::Esc => {
                        app.state = AppState::Results;
                    }
                    KeyCode::Char('q') => {
                        app.running = false;
                    }
                    KeyCode::Backspace | KeyCode::Char('b') => {
                        app.show_downloads_panel = false;
                        app.state = app.previous_app_state;
                    }
                    KeyCode::Char('/') | KeyCode::Char('s') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if let Some(idx) = app.selected_local_file_index {
                            if idx > 0 {
                                app.selected_local_file_index = Some(idx - 1);
                            } else if !app.download_manager.task_order.is_empty() {
                                app.selected_local_file_index = None;
                                app.selected_download_index = Some(app.download_manager.task_order.len() - 1);
                            }
                        } else if let Some(idx) = app.selected_download_index {
                            if idx > 0 {
                                app.selected_download_index = Some(idx - 1);
                            }
                        } else {
                            if !app.local_files.is_empty() {
                                app.selected_local_file_index = Some(0);
                            } else if !app.download_manager.task_order.is_empty() {
                                app.selected_download_index = Some(0);
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if let Some(idx) = app.selected_download_index {
                            if idx < app.download_manager.task_order.len() - 1 {
                                app.selected_download_index = Some(idx + 1);
                            } else if !app.local_files.is_empty() {
                                app.selected_download_index = None;
                                app.selected_local_file_index = Some(0);
                            }
                        } else if let Some(idx) = app.selected_local_file_index {
                            if idx < app.local_files.len().saturating_sub(1) {
                                app.selected_local_file_index = Some(idx + 1);
                            }
                        } else {
                            if !app.download_manager.task_order.is_empty() {
                                app.selected_download_index = Some(0);
                            } else if !app.local_files.is_empty() {
                                app.selected_local_file_index = Some(0);
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(idx) = app.selected_download_index {
                            if app.selected_download_indices.contains(&idx) {
                                app.selected_download_indices.remove(&idx);
                            } else {
                                app.selected_download_indices.insert(idx);
                            }
                        } else if let Some(idx) = app.selected_local_file_index {
                            if app.selected_local_file_indices.contains(&idx) {
                                app.selected_local_file_indices.remove(&idx);
                            } else {
                                app.selected_local_file_indices.insert(idx);
                            }
                        }
                    }
                    KeyCode::Enter => {
                         if app.selected_local_file_index.is_some() || app.selected_download_index.is_some() {
                             app.previous_app_state = app.state;
                             app.state = AppState::ActionMenu;
                         }
                    }
                     KeyCode::Char('p') => {
                        let mut handled = false;
                        if let Some(idx) = app.selected_download_index {
                             if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                  if let Some(task) = app.download_manager.tasks.get(task_id) {
                                      match task.status {
                                          crate::model::download::DownloadStatus::Downloading => {
                                              let _ = app.download_control_tx.send(DownloadControl::Pause(task_id.clone()));
                                          }
                                          crate::model::download::DownloadStatus::Paused => {
                                              let _ = app.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                          }
                                          crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                              let video = task.video.clone();
                                              let format_id = task.format_id.clone();
                                              let _ = app.new_download_tx.send((video, format_id));
                                              
                                              if let Some(t) = app.download_manager.tasks.get_mut(task_id) {
                                                  t.status = crate::model::download::DownloadStatus::Pending;
                                              }
                                          }
                                          _ => {}
                                      }
                                      handled = true;
                                  }
                             }
                        }
                        if !handled {
                            actions::toggle_pause(app);
                        }
                    }
                    KeyCode::Char('x') => {
                         let mut handled = false;
                         if let Some(idx) = app.selected_download_index {
                             if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                 let _ = app.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                 handled = true;
                             }
                         }
                         if !handled {
                             actions::stop_playback(app);
                         }
                    }
                    KeyCode::Left => {
                        actions::seek(app, -5);
                    }
                    KeyCode::Right => {
                        actions::seek(app, 5);
                    }
                    KeyCode::Char('[') => {
                        actions::seek(app, -30);
                    }
                    KeyCode::Char(']') => {
                        actions::seek(app, 30);
                    }
                    _ => {} 
                },
                AppState::ActionMenu => {
                    if code == KeyCode::Esc || code == KeyCode::Char('q') {
                        app.state = app.previous_app_state;
                        return;
                    }

                    if let Some(action) =
                        actions::get_available_actions(app).iter().find(|a| {
                            a.key == code || 
                            (a.action == AppAction::CopyUrlOrId && (code == KeyCode::Char('i') || code == KeyCode::Char('c')))
                        })
                    {
                        let effective_code = if action.action == AppAction::CopyUrlOrId {
                            code
                        } else {
                            action.key
                        };

                        match action.action {
                            AppAction::PlayLocalExternal => {
                                 if let Some(idx) = app.selected_local_file_index {
                                     if let Some((path, name)) = app.local_files.get(idx).map(|f| (f.path.to_string_lossy().to_string(), f.name.clone())) {
                                         actions::stop_playback(app);
                                         app.pending_action = Some((
                                             AppAction::WatchExternal,
                                             path,
                                             name
                                         ));
                                         app.state = app.previous_app_state;
                                     }
                                 }
                            }
                            AppAction::PlayLocalTerminal => {
                                 if let Some(idx) = app.selected_local_file_index {
                                     if let Some(file) = app.local_files.get(idx) {
                                         let path = file.path.to_string_lossy().to_string();
                                         let name = file.name.clone();
                                         let is_audio = file.is_audio();
                                         
                                         actions::stop_playback(app);
                                         
                                         if is_audio {
                                             app.pending_action = Some((
                                                 AppAction::ListenAudio,
                                                 path,
                                                 name
                                             ));
                                         } else {
                                             app.terminal_ready_url = Some(path);
                                         }
                                         app.state = app.previous_app_state;
                                     }
                                 }
                            }
                            AppAction::PlayLocalAudio => {
                                 if let Some(idx) = app.selected_local_file_index {
                                     if let Some((path, name)) = app.local_files.get(idx).map(|f| (f.path.to_string_lossy().to_string(), f.name.clone())) {
                                         actions::stop_playback(app);
                                         app.pending_action = Some((
                                             AppAction::ListenAudio,
                                             path,
                                             name
                                         ));
                                         app.state = app.previous_app_state;
                                     }
                                 }
                            }
                            AppAction::DeleteLocalFile => {
                                 if let Some(idx) = app.selected_local_file_index {
                                     if let Some(file) = app.local_files.get(idx) {
                                         if let Err(e) = local::delete_file(&file.path) {
                                             app.status_message = Some(format!("Error deleting: {}", e));
                                         } else {
                                             app.status_message = Some("File deleted.".to_string());
                                             actions::refresh_local_files(app);
                                         }
                                     }
                                 }
                                 app.state = app.previous_app_state;
                            }
                            AppAction::DeleteSelectedLocalFiles => {
                                 let indices: Vec<usize> = app.selected_local_file_indices.iter().cloned().collect();
                                 for &idx in &indices {
                                     if let Some(file) = app.local_files.get(idx) {
                                         let _ = local::delete_file(&file.path);
                                     }
                                 }
                                 
                                 if let Some(idx) = app.selected_local_file_index {
                                     if indices.contains(&idx) {
                                         app.selected_local_file_index = None;
                                     }
                                 }
                                 
                                 app.selected_local_file_indices.clear();
                                 actions::refresh_local_files(app);
                                 app.status_message = Some(format!("Deleted {} files.", indices.len()));
                                 app.state = app.previous_app_state;
                            }
                            AppAction::ResumeDownload => {
                                 if let Some(idx) = app.selected_download_index {
                                     if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                         if let Some(task) = app.download_manager.tasks.get(task_id) {
                                             match task.status {
                                                 crate::model::download::DownloadStatus::Downloading => {
                                                     let _ = app.download_control_tx.send(DownloadControl::Pause(task_id.clone()));
                                                 }
                                                 crate::model::download::DownloadStatus::Paused => {
                                                     let _ = app.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                                 }
                                                 crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                                     let video = task.video.clone();
                                                     let format_id = task.format_id.clone();
                                                     let _ = app.new_download_tx.send((video, format_id));
                                                     if let Some(t) = app.download_manager.tasks.get_mut(task_id) {
                                                         t.status = crate::model::download::DownloadStatus::Pending;
                                                     }
                                                 }
                                                 _ => {}
                                             }
                                         }
                                     }
                                 }
                                 app.state = app.previous_app_state;
                            }
                            AppAction::ResumeSelectedDownloads => {
                                 let indices: Vec<usize> = app.selected_download_indices.iter().cloned().collect();
                                 for idx in indices {
                                     if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                         if let Some(task) = app.download_manager.tasks.get(task_id) {
                                             match task.status {
                                                 crate::model::download::DownloadStatus::Paused => {
                                                     let _ = app.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                                 }
                                                 crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                                     let video = task.video.clone();
                                                     let format_id = task.format_id.clone();
                                                     let _ = app.new_download_tx.send((video, format_id));
                                                     if let Some(t) = app.download_manager.tasks.get_mut(task_id) {
                                                         t.status = crate::model::download::DownloadStatus::Pending;
                                                     }
                                                 }
                                                 _ => {}
                                             }
                                         }
                                     }
                                 }
                                 app.selected_download_indices.clear();
                                 app.state = app.previous_app_state;
                            }
                            AppAction::CancelDownload => {
                                 if let Some(idx) = app.selected_download_index {
                                     if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                         let _ = app.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                     }
                                 }
                                 app.state = app.previous_app_state;
                            }
                            AppAction::CancelSelectedDownloads => {
                                 let indices: Vec<usize> = app.selected_download_indices.iter().cloned().collect();
                                 for idx in indices {
                                     if let Some(task_id) = app.download_manager.task_order.get(idx) {
                                         let _ = app.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                     }
                                 }
                                 app.selected_download_indices.clear();
                                 app.state = app.previous_app_state;
                            }
                            AppAction::CleanupLocalGarbage => {
                                let download_path_buf = local::resolve_path(&app.download_directory);
                                let download_path = download_path_buf.as_path();
                                match local::cleanup_garbage(download_path) {
                                    Ok(count) => {
                                        app.status_message = Some(format!("Cleaned {} garbage files.", count));
                                        
                                        // Clear tasks that are not actively downloading/paused
                                        // or just clear the whole list as requested to "continue" fresh
                                        app.download_manager.tasks.clear();
                                        app.download_manager.task_order.clear();
                                        app.selected_download_index = None;
                                        app.selected_download_indices.clear();
                                        
                                        actions::refresh_local_files(app);
                                    }
                                    Err(e) => {
                                        app.status_message = Some(format!("Error cleanup: {}", e));
                                    }
                                }
                                app.state = app.previous_app_state;
                            }
                            _ => {
                                 if let Some(idx) = app.selected_result_index {
                                    if let Some(video) = app.search_results.get(idx) {
                                        let url = video.url.clone();
                                        let title = video.title.clone();
                                        match action.action {
                                            AppAction::ViewPlaylist => {
                                                app.status_message =
                                                    Some("Attempting to view playlist...".to_string());
                                                let (query, title) = if video.video_type
                                                    == crate::model::VideoType::Playlist
                                                {
                                                    (
                                                        format!("https://www.youtube.com/playlist?list={}", video.id),
                                                        video.title.clone(),
                                                    )
                                                } else if video.parent_playlist_url.is_some() {
                                                    (
                                                        video.parent_playlist_url.clone().unwrap(),
                                                        video
                                                            .parent_playlist_title
                                                            .clone()
                                                            .unwrap_or_else(|| "Playlist".to_string()),
                                                    )
                                                } else {
                                                    (video.url.clone(), video.title.clone())
                                                };

                                                let parent = video.clone();
                                                let children = std::mem::take(&mut app.search_results);
                                                app.playlist_stack.push((
                                                    parent,
                                                    children,
                                                    app.selected_result_index,
                                                ));
                                                app.selected_playlist_indices.clear();
                                                app.search_results.clear(); 
                                                app.selected_result_index = Some(0); 

                                                app.is_searching = true;
                                                app.search_progress = Some(0.0);
                                                app.current_search_id += 1;
                                                let _ = app.search_tx.send((
                                                    query,
                                                    1,
                                                    app.playlist_limit, 
                                                    app.current_search_id,
                                                    app.show_live,
                                                    app.show_playlists,
                                                ));
                                                app.status_message =
                                                    Some(format!("Loading playlist: {}...", title));
                                                app.state = AppState::Results;
                                                return;
                                            }
                                            AppAction::Download => {
                                                let _ = app.format_tx.send(url);
                                                app.input_mode = InputMode::Loading;
                                                app.status_message =
                                                    Some("Fetching formats...".to_string());
                                            }
                                            AppAction::WatchInTerminal => {
                                                actions::stop_playback(app);
                                                actions::start_terminal_loading(app, url, title);
                                                app.state = app.previous_app_state;
                                            }
                                            AppAction::DownloadSelected => {
                                                 let selected_videos: Vec<Video> = app
                                                    .selected_playlist_indices
                                                    .iter()
                                                    .filter_map(|&idx| app.search_results.get(idx).cloned())
                                                    .collect();


                                                if selected_videos.is_empty() {
                                                    app.status_message =
                                                        Some("No videos selected.".to_string());
                                                } else {
                                                    for video in selected_videos {
                                                         app.download_manager.add_task(&video, "best");
                                                        let _ = app
                                                            .new_download_tx
                                                            .send((video, "best".to_string()));
                                                    }
                                                    app.status_message =
                                                        Some("Starting downloads...".to_string());
                                                    app.state = app.previous_app_state;
                                                }
                                            }
                                            AppAction::DownloadPlaylist => {
                                                if let Some(_parent_url) = &video.parent_playlist_url {
                                                    app.status_message = Some("Playlist download from this context is not fully implemented. Downloading current view.".to_string());
                                                }
                                                let videos: Vec<Video> = app
                                                    .search_results
                                                    .iter().cloned()
                                                    .collect();
                                                for video in videos {
                                                    app.download_manager.add_task(&video, "best");
                                                    let _ = app
                                                        .new_download_tx
                                                        .send((video, "best".to_string()));
                                                }
                                                app.status_message =
                                                    Some("Starting playlist download...".to_string());

                                                app.state = app.previous_app_state;
                                            }
                                            AppAction::CopyUrlOrId => {
                                                let text_to_copy = if effective_code == KeyCode::Char('i') {
                                                    video.id.clone()
                                                } else {
                                                    video.url.clone()
                                                };
                                                let label = if effective_code == KeyCode::Char('i') { "ID" } else { "URL" };

                                                if let Some(clipboard) = &mut app.clipboard {
                                                    if clipboard.set_text(text_to_copy).is_ok() {
                                                        app.status_message = Some(format!("{} copied to clipboard.", label));
                                                    } else {
                                                        app.status_message = Some(format!("Failed to copy {}.", label));
                                                    }
                                                } else {
                                                    app.status_message = Some("Clipboard not available.".to_string());
                                                }
                                                app.state = app.previous_app_state;
                                            }
                                            AppAction::OpenInBrowser => {
                                                let target_url = if app.playlist_stack.is_empty() {
                                                    video.parent_playlist_url.as_ref().unwrap_or(&url)
                                                } else {
                                                    &url
                                                };
                                                if webbrowser::open(target_url).is_ok() {
                                                    app.status_message = Some("Opening in browser...".to_string());
                                                } else {
                                                    app.status_message = Some("Failed to open browser.".to_string());
                                                }
                                                app.state = app.previous_app_state;
                                            }
                                            _ => {
                                                app.pending_action = Some((action.action, url, title));
                                                app.state = app.previous_app_state;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => match code {
                    KeyCode::Char('q') => {
                        app.running = false;
                    }
                    KeyCode::Tab => {
                        if app.show_downloads_panel {
                            app.state = AppState::Downloads;
                        }
                    }
                    KeyCode::Char('d') => {
                        if app.show_downloads_panel {
                            app.show_downloads_panel = false;
                            app.state = app.previous_app_state;
                        } else {
                            app.show_downloads_panel = true;
                            app.previous_app_state = app.state;
                            app.state = AppState::Downloads;
                            actions::refresh_local_files(app); 
                            
                            if !app.download_manager.task_order.is_empty() {
                                app.selected_download_index = Some(0);
                                app.selected_local_file_index = None;
                            } else if !app.local_files.is_empty() {
                                app.selected_download_index = None;
                                app.selected_local_file_index = Some(0);
                            }
                        }
                    }
                    KeyCode::Char('/') | KeyCode::Char('s') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        updates::move_selection(app, 1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        updates::move_selection(app, -1);
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = app.selected_result_index {
                            if idx < app.search_results.len() {
                                app.previous_app_state = app.state;
                                app.state = AppState::ActionMenu;
                            } else if !app.is_url_mode {
                                actions::load_more(app);
                            }
                        }
                    }
                    KeyCode::Backspace | KeyCode::Char('b') => {
                        if let Some((_parent, children, prev_idx)) = app.playlist_stack.pop() {
                            app.search_results = children;
                            app.selected_result_index = prev_idx;
                            app.selected_playlist_indices.clear();
                            app.status_message =
                                Some("Returned to search results.".to_string());
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(idx) = app.selected_result_index {
                            if idx < app.search_results.len() {
                                if app.selected_playlist_indices.contains(&idx) {
                                    app.selected_playlist_indices.remove(&idx);
                                } else {
                                    app.selected_playlist_indices.insert(idx);
                                }
                            }
                        }
                    }
                    KeyCode::Char('x') => {
                        actions::stop_playback(app);
                    }
                    KeyCode::Char('p') => {
                        actions::toggle_pause(app);
                    }
                    KeyCode::Left => {
                        actions::seek(app, -5);
                    }
                    KeyCode::Right => {
                        actions::seek(app, 5);
                    }
                    KeyCode::Char('[') => {
                        actions::seek(app, -30);
                    }
                    KeyCode::Char(']') => {
                        actions::seek(app, 30);
                    }
                    _ => {} 
                },
            }
        }
        InputMode::Editing => {
            let control = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Enter => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        let val = app.settings_input.clone();
                        match app.settings_editing_item {
                            Some(crate::tui::components::settings::SettingItem::SearchLimit) => {
                                if let Ok(n) = val.parse::<u32>() {
                                    app.search_limit = n;
                                    app.status_message = Some(format!("Search Limit set to {}", n));
                                    app.save_config();
                                }
                            }
                            Some(crate::tui::components::settings::SettingItem::PlaylistLimit) => {
                                if let Ok(n) = val.parse::<u32>() {
                                    app.playlist_limit = n;
                                    app.status_message = Some(format!("Playlist Limit set to {}", n));
                                    app.save_config();
                                }
                            }
                            Some(crate::tui::components::settings::SettingItem::DownloadDirectory) => {
                                app.download_directory = val;
                                app.status_message = Some(format!("Download Directory set to {}", app.download_directory));
                                app.save_config();
                            }
                            _ => {}
                        }
                        app.settings_input.clear();
                        app.settings_cursor_position = 0;
                        app.settings_editing_item = None;
                        app.input_mode = InputMode::Normal;
                    } else {
                        actions::perform_search(app);
                    }
                }
                KeyCode::Char(c) => {
                    if control {
                        match c {
                            'u' => {
                                if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                                    app.settings_input.drain(..app.settings_cursor_position);
                                    app.settings_cursor_position = 0;
                                } else {
                                    app.search_query.drain(..app.cursor_position);
                                    app.cursor_position = 0;
                                }
                            }
                            'k' => {
                                if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                                    app.settings_input.truncate(app.settings_cursor_position);
                                } else {
                                    app.search_query.truncate(app.cursor_position);
                                }
                            }
                            'w' | 'h' => {
                                delete_word_backwards(app);
                            }
                            'a' => {
                                if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                                    app.settings_cursor_position = 0;
                                } else {
                                    app.cursor_position = 0;
                                }
                            }
                            'e' => {
                                if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                                    app.settings_cursor_position = app.settings_input.len();
                                } else {
                                    app.cursor_position = app.search_query.len();
                                }
                            }
                            _ => {} 
                        }
                    } else {
                        if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                            app.settings_input.insert(app.settings_cursor_position, c);
                            app.settings_cursor_position += 1;
                        } else {
                            app.search_query.insert(app.cursor_position, c);
                            app.cursor_position += 1;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if control {
                        delete_word_backwards(app);
                    } else {
                        if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                            if app.settings_cursor_position > 0 {
                                app.settings_input.remove(app.settings_cursor_position - 1);
                                app.settings_cursor_position -= 1;
                            }
                        } else if app.cursor_position > 0 {
                            app.search_query.remove(app.cursor_position - 1);
                            app.cursor_position -= 1;
                        }
                    }
                }
                KeyCode::Delete => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        if app.settings_cursor_position < app.settings_input.len() {
                            app.settings_input.remove(app.settings_cursor_position);
                        }
                    } else if app.cursor_position < app.search_query.len() {
                        app.search_query.remove(app.cursor_position);
                    }
                }
                KeyCode::Left => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        if app.settings_cursor_position > 0 {
                            app.settings_cursor_position -= 1;
                        }
                    } else if app.cursor_position > 0 {
                        app.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        if app.settings_cursor_position < app.settings_input.len() {
                            app.settings_cursor_position += 1;
                        }
                    } else if app.cursor_position < app.search_query.len() {
                        app.cursor_position += 1;
                    }
                }
                KeyCode::Home => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        app.settings_cursor_position = 0;
                    } else {
                        app.cursor_position = 0;
                    }
                }
                KeyCode::End => {
                    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
                        app.settings_cursor_position = app.settings_input.len();
                    } else {
                        app.cursor_position = app.search_query.len();
                    }
                }
                KeyCode::Esc | KeyCode::Tab => {
                    if app.state == AppState::Settings {
                        app.settings_input.clear();
                        app.settings_cursor_position = 0;
                        app.settings_editing_item = None;
                    }
                    app.input_mode = InputMode::Normal;
                }
                _ => {} 
            }
        }
        InputMode::Loading => {
            if code == KeyCode::Esc || code == KeyCode::Char('x') {
                app.terminal_loading = false;
                app.terminal_loading_error = None;
                app.input_mode = InputMode::Normal;
            }
        }
    }
}

fn delete_word_backwards(app: &mut App) {
    if app.state == AppState::Settings && app.settings_editing_item.is_some() {
        if app.settings_cursor_position == 0 {
            return;
        }

        let mut chars = app.settings_input[..app.settings_cursor_position]
            .char_indices()
            .rev()
            .peekable();

        while let Some((_, c)) = chars.next() {
            if !c.is_whitespace() {
                break;
            }
        }

        let mut start_idx = 0;
        while let Some((idx, c)) = chars.next() {
            if c.is_whitespace() {
                start_idx = idx + 1;
                break;
            }
        }

        app.settings_input.drain(start_idx..app.settings_cursor_position);
        app.settings_cursor_position = start_idx;
    } else {
        if app.cursor_position == 0 {
            return;
        }

        let mut chars = app.search_query[..app.cursor_position]
            .char_indices()
            .rev()
            .peekable();

        while let Some((_, c)) = chars.next() {
            if !c.is_whitespace() {
                break;
            }
        }

        let mut start_idx = 0;
        while let Some((idx, c)) = chars.next() {
            if c.is_whitespace() {
                start_idx = idx + 1;
                break;
            }
        }

        app.search_query.drain(start_idx..app.cursor_position);
        app.cursor_position = start_idx;
    }
}
