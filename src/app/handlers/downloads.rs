use crate::app::actions;
use crate::app::state::DownloadDialogMode;
use crate::app::{App, AppAction, AppState, DownloadControl, InputMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_scroll(app: &mut App, dir: i32) {
    if dir < 0 {
        // Scroll up
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
    } else {
        // Scroll down
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
}

pub fn handle_key(app: &mut App, code: KeyCode) {
    match code {
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
            if let Some(idx) = app.selected_download_index
                && let Some(task_id) = app.download_manager.task_order.get(idx)
                && let Some(task) = app.download_manager.tasks.get(task_id)
            {
                match task.status {
                    crate::model::download::DownloadStatus::Downloading => {
                        let _ = app
                            .download_control_tx
                            .send(DownloadControl::Pause(task_id.clone()));
                    }
                    crate::model::download::DownloadStatus::Paused => {
                        let _ = app
                            .download_control_tx
                            .send(DownloadControl::Resume(task_id.clone()));
                    }
                    crate::model::download::DownloadStatus::Canceled
                    | crate::model::download::DownloadStatus::Error(_) => {
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
            if !handled {
                actions::toggle_pause(app);
            }
        }
        KeyCode::Char('x') => {
            let mut handled = false;
            if let Some(idx) = app.selected_download_index
                && let Some(task_id) = app.download_manager.task_order.get(idx)
            {
                let _ = app
                    .download_control_tx
                    .send(DownloadControl::Cancel(task_id.clone()));
                handled = true;
            }
            if !handled {
                actions::stop_playback(app);
            }
        }
        KeyCode::Left => {
            actions::seek(app, -5);
        }
        KeyCode::Right => {
            if !app.playback_is_audio
                && let Some(idx) = app.selected_local_file_index
                && let Some(file) = app.local_files.get(idx)
            {
                let path = file.path.to_string_lossy().to_string();
                let name = file.name.clone();
                actions::stop_playback(app);
                app.pending_action = Some((AppAction::WatchExternal, path, name));
                return;
            }
            actions::seek(app, 5);
        }
        KeyCode::Char('[') => {
            actions::seek(app, -30);
        }
        KeyCode::Char(']') => {
            actions::seek(app, 30);
        }
        _ => {}
    }
}

pub fn handle_download_dialog_key(app: &mut App, code: KeyCode) {
    let item_count: usize = match app.download_dialog_mode {
        DownloadDialogMode::Single => 3,
        DownloadDialogMode::BulkSelected | DownloadDialogMode::BulkAll => 2,
    };

    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.state = AppState::ActionMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.download_dialog_index > 0 {
                app.download_dialog_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.download_dialog_index < item_count.saturating_sub(1) {
                app.download_dialog_index += 1;
            }
        }
        KeyCode::Enter => {
            let format_id = match app.download_dialog_index {
                0 => "best",       // Video
                1 => "audio_best", // Audio
                _ => "",
            };

            match app.download_dialog_mode {
                DownloadDialogMode::Single => {
                    if app.download_dialog_index == 2 {
                        // "Select Format..." — proceed with format selection
                        if let Some(video) = app.action_video.clone() {
                            let url = video.url;
                            let _ = app.format_tx.send(url);
                            app.input_mode = InputMode::Loading;
                            app.status_message = Some("Fetching formats...".to_string());
                            app.format_selection_mode =
                                crate::app::state::FormatSelectionMode::Download;
                        }
                    } else if let Some(video) = app.action_video.clone() {
                        app.download_manager.add_task(&video, format_id);
                        let _ = app
                            .new_download_tx
                            .send((video.clone(), format_id.to_string()));
                        app.status_message = Some("Download started...".to_string());
                        app.action_video = None;
                        app.state = app.previous_app_state;
                    }
                }
                DownloadDialogMode::BulkSelected => {
                    let selected_videos: Vec<crate::model::Video> = app
                        .selected_playlist_indices
                        .iter()
                        .filter_map(|&idx| app.search_results.get(idx).cloned())
                        .collect();

                    if selected_videos.is_empty() {
                        app.status_message = Some("No videos selected.".to_string());
                    } else {
                        for video in selected_videos {
                            app.download_manager.add_task(&video, format_id);
                            let _ = app.new_download_tx.send((video, format_id.to_string()));
                        }
                        app.status_message = Some("Starting downloads...".to_string());
                    }
                    app.state = app.previous_app_state;
                }
                DownloadDialogMode::BulkAll => {
                    let videos: Vec<crate::model::Video> = app.search_results.to_vec();
                    for video in videos {
                        app.download_manager.add_task(&video, format_id);
                        let _ = app.new_download_tx.send((video, format_id.to_string()));
                    }
                    app.status_message = Some("Starting downloads...".to_string());
                    app.state = app.previous_app_state;
                }
            }
        }
        _ => {}
    }
}

pub fn handle_download_dialog_mouse(app: &mut App, x: u16, y: u16, double_click: bool) {
    if let Some(area) = app.download_dialog_area {
        if super::is_in_rect(x, y, area) {
            let item_count = match app.download_dialog_mode {
                DownloadDialogMode::Single => 3,
                DownloadDialogMode::BulkSelected | DownloadDialogMode::BulkAll => 2,
            };
            let header_height = 2;
            let list_start_y = area.y + 1 + header_height;
            if y >= list_start_y {
                let relative_y = y - list_start_y;
                if (relative_y as usize) < item_count {
                    app.download_dialog_index = relative_y as usize;
                    if double_click {
                        super::handle_key_event(
                            app,
                            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
                        );
                    }
                }
            }
        } else {
            // Click outside -> back to action menu
            app.state = AppState::ActionMenu;
        }
    }
}
