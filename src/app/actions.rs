use super::{
    App,
    AppAction,
    AppState,
    InputMode,
    Action
};
use crate::sys::{local, yt};
use crossterm::event::KeyCode;

pub fn refresh_local_files(app: &mut App) {
    let download_path_buf = local::resolve_path(&app.download_directory);
    let download_path = download_path_buf.as_path();
    app.local_files = local::scan_local_files(download_path);
    if !app.local_files.is_empty() {
         if app.selected_local_file_index.is_none() {
             app.selected_local_file_index = Some(0);
         } else if let Some(idx) = app.selected_local_file_index {
             if idx >= app.local_files.len() {
                 app.selected_local_file_index = Some(app.local_files.len().saturating_sub(1));
             }
         }
    } else {
        app.selected_local_file_index = None;
    }
}

pub fn get_available_actions(app: &App) -> Vec<Action> {
    let mut actions = Vec::new();

    let current_context = if app.state == AppState::ActionMenu {
        app.previous_app_state
    } else {
        app.state
    };

    // Local Actions (High Priority if in Downloads view)
    if current_context == AppState::Downloads {
        // Actions for Local Files
        if let Some(idx) = app.selected_local_file_index {
             if let Some(file) = app.local_files.get(idx) {
                 if !file.is_garbage {
                    actions.push(Action::new(
                        KeyCode::Char('w'),
                        "Play (External)",
                        AppAction::PlayLocalExternal,
                    ));
                    actions.push(Action::new(
                        KeyCode::Char('t'),
                        "Play (Terminal/Default)",
                        AppAction::PlayLocalTerminal, 
                    ));
                     actions.push(Action::new(
                        KeyCode::Char('a'),
                        "Play (Audio)",
                        AppAction::PlayLocalAudio,
                    ));
                 }
                 actions.push(Action::new(
                    KeyCode::Char('x'),
                    "Delete File",
                    AppAction::DeleteLocalFile,
                ));
             }
        }
        // Actions for Active Downloads
        if let Some(idx) = app.selected_download_index {
            if let Some(task_id) = app.download_manager.task_order.get(idx) {
                if let Some(task) = app.download_manager.tasks.get(task_id) {
                    match task.status {
                        crate::model::download::DownloadStatus::Downloading | crate::model::download::DownloadStatus::Pending => {
                            actions.push(Action::new(
                                KeyCode::Char('p'),
                                "Pause Download",
                                AppAction::ResumeDownload, 
                            ));
                        }
                        crate::model::download::DownloadStatus::Paused => {
                            actions.push(Action::new(
                                KeyCode::Char('p'),
                                "Resume Download",
                                AppAction::ResumeDownload,
                            ));
                        }
                        crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                            actions.push(Action::new(
                                KeyCode::Char('p'),
                                "Restart Download",
                                AppAction::ResumeDownload,
                            ));
                        }
                        _ => {} 
                    }
                }
            }

             actions.push(Action::new(
                KeyCode::Char('x'),
                "Cancel Download",
                AppAction::CancelDownload,
            ));
        }

        if !app.selected_download_indices.is_empty() {
             actions.push(Action::new(
                KeyCode::Char('P'),
                "Resume/Restart Selected",
                AppAction::ResumeSelectedDownloads,
            ));
             actions.push(Action::new(
                KeyCode::Char('X'),
                "Cancel Selected Downloads",
                AppAction::CancelSelectedDownloads,
            ));
        }

         if !app.selected_local_file_indices.is_empty() {
             actions.push(Action::new(
                KeyCode::Char('d'),
                "Delete Selected",
                AppAction::DeleteSelectedLocalFiles,
            ));
         }
         actions.push(Action::new(
            KeyCode::Char('c'),
            "Cleanup Garbage (.part, .ytdl)",
            AppAction::CleanupLocalGarbage,
        ));
                    
        return actions;
    }


    if let Some(idx) = app.selected_result_index {
        if let Some(video) = app.search_results.get(idx) {
            if video.video_type == crate::model::VideoType::Playlist {
                actions.push(Action::new(
                    KeyCode::Enter,
                    "Open Playlist",
                    AppAction::ViewPlaylist,
                ));
                actions.push(Action::new(
                    KeyCode::Char('l'),
                    "Download All (Playlist)",
                    AppAction::DownloadPlaylist,
                ));
            } else {
                actions.push(Action::new(
                    KeyCode::Char('w'),
                    "Watch (External)",
                    AppAction::WatchExternal,
                ));
                actions.push(Action::new(
                    KeyCode::Char('t'),
                    "Watch (In Terminal)",
                    AppAction::WatchInTerminal,
                ));
                actions.push(Action::new(
                    KeyCode::Char('a'),
                    "Listen (Audio Only)",
                    AppAction::ListenAudio,
                ));
                actions.push(Action::new(
                    KeyCode::Char('d'),
                    "Download",
                    AppAction::Download,
                ));

                // If this video belongs to a playlist, add playlist options
                if video.parent_playlist_id.is_some() {
                    actions.push(Action::new(
                        KeyCode::Char('p'),
                        "Open Parent Playlist",
                        AppAction::ViewPlaylist,
                    ));
                    actions.push(Action::new(
                        KeyCode::Char('l'),
                        "Download All (Parent Playlist)",
                        AppAction::DownloadPlaylist,
                    ));
                }
            }
        }
    }

    if !app.selected_playlist_indices.is_empty() {
        actions.push(Action::new(
            KeyCode::Char('s'),
            "Download Selected",
            AppAction::DownloadSelected,
        ));
    }

    if !app.playlist_stack.is_empty() {
        if !actions
            .iter()
            .any(|a| a.action == AppAction::DownloadPlaylist)
        {
            actions.push(Action::new(
                KeyCode::Char('l'),
                "Download All (Current View)",
                AppAction::DownloadPlaylist,
            ));
        }
    }

    actions
}

pub fn perform_search(app: &mut App) {
    if app.search_query.trim().is_empty() {
        return;
    }

    app.input_mode = InputMode::Normal;
    app.search_results.clear();
    app.pending_resolution_ids.clear();
    app.selected_result_index = None;
    app.playlist_stack.clear();
    app.selected_playlist_indices.clear();
    app.search_progress = Some(0.0);
    app.is_searching = true;
    app.current_search_id += 1;
    app.status_message = Some(format!("Searching for '{}'...", app.search_query));

    let is_url = 
        app.search_query.starts_with("http://") || app.search_query.starts_with("https://");
    app.is_url_mode = is_url;

    let mut is_direct_playlist_url = false;
    if is_url {
        is_direct_playlist_url = app.search_query.contains("list=") 
            || app.search_query.contains("/playlist/");
        if !is_direct_playlist_url &&
           (app.search_query.contains("PL") || app.search_query.contains("UU") ||
            app.search_query.contains("FL") || app.search_query.contains("RD") ||
            app.search_query.contains("OL")) {
            is_direct_playlist_url = true;
        }
    }

    app.search_offset = 1; 

    if is_url && is_direct_playlist_url {
        let _ = app
            .search_tx
            .send((app.search_query.clone(), 1, app.playlist_limit, app.current_search_id, app.show_live, app.show_playlists));
    } else if is_url {
        let _ = app
            .search_tx
            .send((app.search_query.clone(), 1, 1, app.current_search_id, app.show_live, app.show_playlists));
    } else {
        let _ = app
            .search_tx
            .send((app.search_query.clone(), 1, app.search_limit, app.current_search_id, app.show_live, app.show_playlists));
    }
}

pub fn load_more(app: &mut App) {
    if app.is_searching || app.search_query.trim().is_empty() {
        return;
    }

    app.is_searching = true;
    app.search_offset += app.search_limit;
    app.search_progress = Some(0.0);
    app.status_message = Some("Loading more...".to_string());

    let _ = app.search_tx.send((
        app.search_query.clone(),
        app.search_offset,
        app.search_offset + (app.search_limit - 1),
        app.current_search_id,
        app.show_live,
        app.show_playlists,
    ));
}

pub fn handle_paste(app: &mut App, text: String) {
    if app.input_mode == InputMode::Editing {
        app.search_query.insert_str(app.cursor_position, &text);
        app.cursor_position += text.len();
    } else {
        app.input_mode = InputMode::Editing;
        app.search_query.insert_str(app.cursor_position, &text);
        app.cursor_position += text.len();
    }
}

pub fn stop_playback(app: &mut App) {
    if let Some(mut child) = app.playback_process.take() {
        let _ = child.start_kill();
    }
    app.playback_cmd_tx = None;
    app.playback_title = None;
    app.playback_time = 0.0;
    app.playback_total = 0.0;
    app.playback_duration_str = None;
    app.is_paused = false;
    app.is_finishing = false;
    app.terminal_loading = false;
    app.terminal_loading_error = None;
    app.terminal_ready_url = None;
    app.status_message = Some("Stopped.".to_string());
    if let Some(mc) = &mut app.media_controller {
        let _ = mc.set_playback_status(false);
    }
}

pub fn toggle_pause(app: &mut App) {
    if app.playback_cmd_tx.is_some() {
        app.is_paused = !app.is_paused;
        send_command(app, "{\"command\": [\"cycle\", \"pause\"]}\n");
        app.status_message = Some(if app.is_paused {
            "Paused".to_string()
        } else {
            "Resumed".to_string()
        });
        if let Some(mc) = &mut app.media_controller {
            let _ = mc.set_playback_status(!app.is_paused);
        }
    }
}

pub fn seek(app: &mut App, seconds: i32) {
    if app.playback_cmd_tx.is_some() {
        let cmd = format!(
            "{{\"command\": [\"osd-msg-bar\", \"seek\", {}, \"relative\"]}}\n",
            seconds
        );
        send_command(app, &cmd);
        app.status_message = Some(format!("Seeked {}s", seconds));
    }
}

pub fn send_command(app: &App, cmd: &str) {
    if let Some(tx) = &app.playback_cmd_tx {
        let mut command = cmd.to_string();
        if !command.ends_with('\n') {
            command.push('\n');
        }
        let _ = tx.send(command);
    }
}

pub fn start_terminal_loading(app: &mut App, url: String, _title: String) {
    app.terminal_loading = true;
    app.terminal_loading_progress = 0.0;
    app.terminal_loading_error = None;
    app.terminal_ready_url = None;
    let tx = app.terminal_ready_tx.clone();

    tokio::spawn(async move {
        let mut cmd = tokio::process::Command::new("yt-dlp");
        cmd.arg("--user-agent");
        let ua = match cmd.output().await {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => "Mozilla/5.0".to_string(), 
        };

        match yt::get_best_stream_url(&url).await {
            Ok(direct_url) => {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let _ = tx.send(Ok(format!("{}|{}", direct_url, ua)));
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });
}