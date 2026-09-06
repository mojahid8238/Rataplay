use crate::app::actions;
use crate::app::state::DownloadDialogMode;
use crate::app::{App, AppAction, AppState, DownloadControl, InputMode};
use crate::sys::local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_action_menu_mouse(app: &mut App, x: u16, y: u16, _double_click: bool) {
    if let Some(area) = app.action_menu_area {
        if super::is_in_rect(x, y, area) {
            let relative_y = y.saturating_sub(area.y).saturating_sub(1); // -1 for border
            let idx = app.action_menu_state.offset() + relative_y as usize;
            let actions = actions::get_available_actions(app);
            if let Some(action) = actions.get(idx) {
                let code = action.key;
                super::handle_key_event(app, KeyEvent::new(code, KeyModifiers::empty()));
            }
        } else {
            // Click outside -> dismiss
            app.state = app.previous_app_state;
        }
    }
}

pub fn handle_format_selection_mouse(app: &mut App, x: u16, y: u16, double_click: bool) {
    if let Some(area) = app.format_selection_area {
        if super::is_in_rect(x, y, area) {
            let header_height = 2; // Header + Margin
            let list_start_y = area.y + 1 + header_height; // Border + Header
            if y >= list_start_y {
                let relative_y = y - list_start_y;
                let idx = app.format_selection_state.offset() + relative_y as usize;
                if idx < app.formats.len() {
                    app.selected_format_index = Some(idx);
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

pub fn handle_format_selection_scroll(app: &mut App, dir: i32) {
    if dir < 0 {
        if let Some(idx) = app.selected_format_index
            && idx > 0
        {
            app.selected_format_index = Some(idx - 1);
        }
    } else if let Some(idx) = app.selected_format_index
        && idx < app.formats.len().saturating_sub(1)
    {
        app.selected_format_index = Some(idx + 1);
    }
}

pub fn handle_format_selection_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.state = AppState::ActionMenu;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(idx) = app.selected_format_index
                && idx > 0
            {
                app.selected_format_index = Some(idx - 1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(idx) = app.selected_format_index
                && idx < app.formats.len().saturating_sub(1)
            {
                app.selected_format_index = Some(idx + 1);
            }
        }
        KeyCode::Enter => {
            let selected_format_id = if let Some(idx) = app.selected_format_index {
                if let Some(fmt) = app.formats.get(idx) {
                    // Construct format string for mpv --ytdl-format
                    // Logic:
                    // 1. If format has no audio, we MUST merge with bestaudio.
                    // 2. We append "/best" as a fallback if the specific format fails.

                    let format_expr = if !fmt.has_audio {
                        format!("{}+bestaudio/bestvideo+bestaudio/best", fmt.format_id)
                    } else {
                        format!("{}/bestvideo+bestaudio/best", fmt.format_id)
                    };

                    Some(format_expr)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(format_id) = selected_format_id
                && let Some(video) = app.action_video.clone()
            {
                match app.format_selection_mode {
                    crate::app::state::FormatSelectionMode::Download => {
                        // Add to manager and start download
                        app.download_manager.add_task(&video, &format_id);
                        let _ = app.new_download_tx.send((video.clone(), format_id));

                        if app.previous_app_state == AppState::Downloads {
                            app.state = AppState::Downloads;
                        } else {
                            app.state = AppState::Results;
                        }
                        app.status_message = Some("Download started...".to_string());
                        app.action_video = None;
                        return;
                    }
                    crate::app::state::FormatSelectionMode::Watch => {
                        let url = video.url.clone();
                        let title = video.title.clone();

                        actions::stop_playback(app);

                        let stored_url = format!("{}::{}", url, format_id);
                        app.pending_action = Some((AppAction::WatchExternal, stored_url, title));

                        if app.previous_app_state == AppState::Downloads {
                            app.state = AppState::Downloads;
                        } else {
                            app.state = AppState::Results;
                        }
                        app.action_video = None;
                        return;
                    }
                }
            }

            // Fallback if video lost or format aborted
            if app.previous_app_state == AppState::Downloads {
                app.state = AppState::Downloads;
            } else {
                app.state = AppState::Results;
            }
        }
        _ => {}
    }
}

pub fn handle_action_menu_key(app: &mut App, code: KeyCode) {
    if code == KeyCode::Esc || code == KeyCode::Char('q') {
        app.state = app.previous_app_state;
        return;
    }

    if let Some(action) = actions::get_available_actions(app).iter().find(|a| {
        a.key == code
            || (a.action == AppAction::CopyUrlOrId
                && (code == KeyCode::Char('i') || code == KeyCode::Char('c')))
    }) {
        let effective_code = if action.action == AppAction::CopyUrlOrId {
            code
        } else {
            action.key
        };

        match action.action {
            AppAction::PlayLocalExternal => {
                if let Some(idx) = app.selected_local_file_index
                    && let Some((path, name)) = app
                        .local_files
                        .get(idx)
                        .map(|f| (f.path.to_string_lossy().to_string(), f.name.clone()))
                {
                    actions::stop_playback(app);
                    app.pending_action = Some((AppAction::WatchExternal, path, name));
                    app.state = app.previous_app_state;
                }
            }
            AppAction::PlayLocalTerminal => {
                if let Some(idx) = app.selected_local_file_index
                    && let Some(file) = app.local_files.get(idx)
                {
                    let path = file.path.to_string_lossy().to_string();
                    let name = file.name.clone();
                    let is_audio = file.is_audio();

                    actions::stop_playback(app);

                    if is_audio {
                        app.pending_action = Some((AppAction::ListenAudio, path, name));
                    } else {
                        app.playback_is_terminal = true;
                        app.terminal_ready_url = Some(path);
                    }
                    app.state = app.previous_app_state;
                }
            }
            AppAction::PlayLocalAudio => {
                if let Some(idx) = app.selected_local_file_index
                    && let Some((path, name)) = app
                        .local_files
                        .get(idx)
                        .map(|f| (f.path.to_string_lossy().to_string(), f.name.clone()))
                {
                    actions::stop_playback(app);
                    app.pending_action = Some((AppAction::ListenAudio, path, name));
                    app.state = app.previous_app_state;
                }
            }
            AppAction::DeleteLocalFile => {
                if let Some(idx) = app.selected_local_file_index
                    && let Some(file) = app.local_files.get(idx)
                {
                    if let Err(e) = local::delete_file(&file.path) {
                        app.status_message = Some(format!("Error deleting: {}", e));
                    } else {
                        app.status_message = Some("File deleted.".to_string());
                        actions::refresh_local_files(app);
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

                if let Some(idx) = app.selected_local_file_index
                    && indices.contains(&idx)
                {
                    app.selected_local_file_index = None;
                }

                app.selected_local_file_indices.clear();
                actions::refresh_local_files(app);
                app.status_message = Some(format!("Deleted {} files.", indices.len()));
                app.state = app.previous_app_state;
            }
            AppAction::ResumeDownload => {
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
                }
                app.state = app.previous_app_state;
            }
            AppAction::ResumeSelectedDownloads => {
                let indices: Vec<usize> = app.selected_download_indices.iter().cloned().collect();
                for idx in indices {
                    if let Some(task_id) = app.download_manager.task_order.get(idx)
                        && let Some(task) = app.download_manager.tasks.get(task_id)
                    {
                        match task.status {
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
                    }
                }
                app.selected_download_indices.clear();
                app.state = app.previous_app_state;
            }
            AppAction::CancelDownload => {
                if let Some(idx) = app.selected_download_index
                    && let Some(task_id) = app.download_manager.task_order.get(idx).cloned()
                {
                    let (is_active, json_path) =
                        if let Some(task) = app.download_manager.tasks.get(&task_id) {
                            (
                                matches!(
                                    task.status,
                                    crate::model::download::DownloadStatus::Downloading
                                        | crate::model::download::DownloadStatus::Pending
                                        | crate::model::download::DownloadStatus::Paused
                                ),
                                task.info_json_path.clone(),
                            )
                        } else {
                            (false, None)
                        };

                    if is_active {
                        let _ = app
                            .download_control_tx
                            .send(DownloadControl::Cancel(task_id));
                    } else {
                        // Delete files (json + part)
                        if let Some(path) = json_path {
                            let _ = crate::sys::local::delete_task_files(&path);
                        }
                        // Remove from memory
                        app.download_manager.tasks.remove(&task_id);
                        app.download_manager.task_order.retain(|id| id != &task_id);
                        app.selected_download_index = None;
                    }
                }
                app.state = app.previous_app_state;
            }
            AppAction::CancelSelectedDownloads => {
                let indices: Vec<usize> = app.selected_download_indices.iter().cloned().collect();
                for idx in indices {
                    if let Some(task_id) = app.download_manager.task_order.get(idx).cloned() {
                        let (is_active, json_path) =
                            if let Some(task) = app.download_manager.tasks.get(&task_id) {
                                (
                                    matches!(
                                        task.status,
                                        crate::model::download::DownloadStatus::Downloading
                                            | crate::model::download::DownloadStatus::Pending
                                            | crate::model::download::DownloadStatus::Paused
                                    ),
                                    task.info_json_path.clone(),
                                )
                            } else {
                                (false, None)
                            };

                        if is_active {
                            let _ = app
                                .download_control_tx
                                .send(DownloadControl::Cancel(task_id));
                        } else {
                            if let Some(path) = json_path {
                                let _ = crate::sys::local::delete_task_files(&path);
                            }
                            app.download_manager.tasks.remove(&task_id);
                            app.download_manager.task_order.retain(|id| id != &task_id);
                        }
                    }
                }
                app.selected_download_indices.clear();
                app.selected_download_index = None;
                app.state = app.previous_app_state;
            }
            AppAction::CleanupLocalGarbage => {
                let download_path_buf = local::resolve_path(&app.download_directory);
                let download_path = download_path_buf.as_path();
                match local::cleanup_garbage(download_path) {
                    Ok(count) => {
                        app.status_message = Some(format!("Cleaned {} garbage files.", count));

                        actions::refresh_local_files(app);
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Error cleanup: {}", e));
                    }
                }
                app.state = app.previous_app_state;
            }
            _ => {
                let effective_state = if app.state == AppState::ActionMenu {
                    app.previous_app_state
                } else {
                    app.state
                };

                let target_video = if effective_state == AppState::Downloads {
                    if let Some(idx) = app.selected_download_index {
                        if let Some(task_id) = app.download_manager.task_order.get(idx) {
                            app.download_manager
                                .tasks
                                .get(task_id)
                                .map(|t| t.video.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    if let Some(idx) = app.selected_result_index {
                        app.search_results.get(idx).cloned()
                    } else {
                        None
                    }
                };

                if let Some(video) = target_video {
                    app.action_video = Some(video.clone());
                    let url = video.url.clone();
                    let title = video.title.clone();
                    match action.action {
                        AppAction::ViewPlaylist => {
                            app.status_message = Some("Attempting to view playlist...".to_string());
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
                            app.playlist_stack
                                .push((parent, children, app.selected_result_index));
                            app.selected_playlist_indices.clear();
                            app.search_results.clear();
                            app.selected_result_index = Some(0);
                            app.search_offset = 1;
                            app.is_playlist_mode = true;
                            app.is_url_mode = true; // Viewing a specific playlist is effectively URL mode

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
                                app.date_filter_unit,
                            ));
                            app.status_message = Some(format!("Loading playlist: {}...", title));
                            app.state = AppState::Results;
                        }
                        AppAction::Download => {
                            app.download_dialog_mode = DownloadDialogMode::Single;
                            app.download_dialog_index = 0;
                            app.state = AppState::DownloadDialog;
                        }
                        AppAction::WatchExternal => {
                            // Start Format Selection instead of direct play
                            let _ = app.format_tx.send(url);
                            app.input_mode = InputMode::Loading;
                            app.status_message = Some("Fetching formats...".to_string());
                            app.format_selection_mode =
                                crate::app::state::FormatSelectionMode::Watch;
                            // state transition will happen in updates.rs when formats arrive
                        }
                        AppAction::WatchInTerminal => {
                            actions::stop_playback(app);
                            actions::start_terminal_loading(app, url, title);
                            app.state = app.previous_app_state;
                        }
                        AppAction::DownloadSelected => {
                            if app.selected_playlist_indices.is_empty() {
                                app.status_message = Some("No videos selected.".to_string());
                                app.state = app.previous_app_state;
                            } else {
                                app.download_dialog_mode = DownloadDialogMode::BulkSelected;
                                app.download_dialog_index = 0;
                                app.state = AppState::DownloadDialog;
                            }
                        }
                        AppAction::DownloadPlaylist => {
                            app.download_dialog_mode = DownloadDialogMode::BulkAll;
                            app.download_dialog_index = 0;
                            app.state = AppState::DownloadDialog;
                        }
                        AppAction::CopyUrlOrId => {
                            let text_to_copy = if effective_code == KeyCode::Char('i') {
                                video.channel_id.clone()
                            } else {
                                video.url.clone()
                            };
                            let label = if effective_code == KeyCode::Char('i') {
                                "Channel ID"
                            } else {
                                "URL"
                            };

                            if let Some(clipboard) = &mut app.clipboard {
                                if clipboard.set_text(text_to_copy).is_ok() {
                                    app.status_message =
                                        Some(format!("{} copied to clipboard.", label));
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
