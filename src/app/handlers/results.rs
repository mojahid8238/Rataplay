use crate::app::actions;
use crate::app::updates;
use crate::app::{App, AppAction, AppState, InputMode};
use crossterm::event::KeyCode;

pub fn handle_scroll(app: &mut App, dir: i32) {
    updates::move_selection(app, dir);
}

pub fn handle_key(app: &mut App, code: KeyCode) {
    match code {
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
                } else if !app.is_url_mode || app.is_playlist_mode {
                    actions::load_more(app);
                }
            }
        }
        KeyCode::Backspace | KeyCode::Char('b') => {
            if let Some((_parent, children, prev_idx)) = app.playlist_stack.pop() {
                app.search_results = children;
                app.selected_result_index = prev_idx;
                app.selected_playlist_indices.clear();
                app.is_playlist_mode = false;
                app.is_url_mode = app.search_query.starts_with("http");
                app.status_message = Some("Returned to search results.".to_string());
            }
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = app.selected_result_index
                && idx < app.search_results.len()
            {
                if app.selected_playlist_indices.contains(&idx) {
                    app.selected_playlist_indices.remove(&idx);
                } else {
                    app.selected_playlist_indices.insert(idx);
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
            if !app.playback_is_audio
                && let Some(idx) = app.selected_result_index
                && idx < app.search_results.len()
                && let Some(video) = app.search_results.get(idx).cloned()
                && video.video_type != crate::model::VideoType::Playlist
            {
                actions::stop_playback(app);
                app.pending_action = Some((
                    AppAction::WatchExternal,
                    format!("{}::best", video.url),
                    video.title,
                ));
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
