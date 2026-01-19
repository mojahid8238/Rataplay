use super::{
    App,
    AppState,
    InputMode
};
use crate::sys::yt;
use crate::sys::media::MediaEvent;
use crate::model::download::DownloadEvent;
use super::actions;

pub fn on_tick(app: &mut App) {
    // check for search results
    while let Ok(result) = app.result_rx.try_recv() {
        match result {
            Ok((item, id)) => {
                if id != app.current_search_id {
                    continue;
                }
                match item {
                    yt::SearchResult::Video(video) => {
                        // Trigger image download if thumbnail exists, even if partial
                        if let Some(url) = &video.thumbnail_url {
                            if !app.image_cache.contains(&video.id) {
                                let _ = app.image_tx.send((video.id.clone(), url.clone()));
                            }
                        }

                        app.search_results.push(video);
                        if app.selected_result_index.is_none() {
                            app.selected_result_index = Some(0);
                            request_image_for_selection(app);
                        }
                        if app.state == AppState::Search {
                            app.state = AppState::Results;
                        }
                    }
                    yt::SearchResult::Progress(progress) => {
                        app.search_progress = Some(progress);
                        if progress >= 1.0 {
                            app.is_searching = false;
                            app.search_progress = None;
                            if app.search_results.is_empty() {
                                app.status_message = Some("No results found.".to_string());
                            } else {
                                app.status_message = Some("Results updated.".to_string());
                            }

                            // Flush pending resolutions
                            if !app.pending_resolution_ids.is_empty() {
                                let items: Vec<String> = 
                                    app.pending_resolution_ids.drain(..).collect();
                                let _ = app.details_tx.send(items);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                app.is_searching = false;
                app.status_message = Some(format!("Error: {}", e));
                app.search_progress = None;
            }
        }
    }

    // Resolve details for the currently selected item if it's partial
    if let Some(idx) = app.selected_result_index {
        if let Some(video) = app.search_results.get(idx) {
            if video.is_partial && video.video_type == crate::model::VideoType::Video {
                if !app.pending_resolution_ids.contains(&video.url) {
                    app.pending_resolution_ids.push(video.url.clone());
                    let _ = app.details_tx.send(vec![video.url.clone()]);
                }
            }
        }
    }

    // Apply resolved details
    while let Ok(res) = app.details_rx.try_recv() {
        match res {
            Ok(v) => {
                let url = v.url.clone();
                // Find and replace in search_results
                if let Some(existing) = app.search_results.iter_mut().find(|x| x.id == v.id) {
                    *existing = v;
                }
                // Remove from pending
                app.pending_resolution_ids.retain(|x| x != &url);
                
                // Trigger image request for selection again if needed
                request_image_for_selection(app);
            }
            Err(e) => {
                app.status_message = Some(format!("Details error: {}", e));
            }
        }
    }

    // check for images 
    while let Ok((id, img)) = app.image_rx.try_recv() {
        app.image_cache.put(id, img);
    }

    // Check for formats
    if let Ok(res) = app.format_rx.try_recv() {
        match res {
            Ok(formats) => {
                app.formats = formats;
                if !app.formats.is_empty() {
                    app.selected_format_index = Some(0);
                }
                // If we were waiting (Loading), switch to FormatSelection
                if app.input_mode == InputMode::Loading {
                    app.input_mode = InputMode::Normal;
                    app.state = AppState::FormatSelection;
                }
            }
            Err(e) => {
                app.input_mode = InputMode::Normal;
                app.status_message = Some(format!("Error fetching formats: {}", e));
            }
        }
    }

    // Check for download events
    while let Ok(event) = app.download_event_rx.try_recv() {
        match event {
            DownloadEvent::Update(id, progress, speed, eta, total_size) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    task.status = crate::model::download::DownloadStatus::Downloading;
                    task.progress = progress;
                    task.speed = speed;
                    task.eta = eta;
                    task.total_size = total_size;
                }
            }
            DownloadEvent::Finished(id) => {
                app.download_manager.tasks.remove(&id);
                app.download_manager.task_order.retain(|x| x != &id);
                app.selected_download_indices.clear();
                app.selected_download_index = None;
                actions::refresh_local_files(app);
            }
            DownloadEvent::Error(id, error) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    if task.status != crate::model::download::DownloadStatus::Canceled {
                        task.status = crate::model::download::DownloadStatus::Error(error);
                    }
                }
            }
            DownloadEvent::Started(id, pid) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    task.pid = Some(pid);
                }
                actions::refresh_local_files(app);
            }
            DownloadEvent::Pause(id) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    task.status = crate::model::download::DownloadStatus::Paused;
                }
            }
            DownloadEvent::Resume(id) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    task.status = crate::model::download::DownloadStatus::Downloading;
                }
            }
            DownloadEvent::Canceled(id) => {
                if let Some(task) = app.download_manager.tasks.get_mut(&id) {
                    task.status = crate::model::download::DownloadStatus::Canceled;
                    task.speed = String::new();
                    task.eta = String::new();
                }
                actions::refresh_local_files(app);
            }
        }
    }

    // Check for media events
    while let Ok(event) = app.media_rx.try_recv() {
        match event {
            MediaEvent::Play => {
                actions::send_command(app, "{\"command\": [\"set_property\", \"pause\", false]}\n");
                app.is_paused = false;
                if let Some(mc) = &mut app.media_controller {
                    let _ = mc.set_playback_status(true);
                }
            }
            MediaEvent::Pause => {
                actions::send_command(app, "{\"command\": [\"set_property\", \"pause\", true]}\n");
                app.is_paused = true;
                if let Some(mc) = &mut app.media_controller {
                    let _ = mc.set_playback_status(false);
                }
            }
            MediaEvent::Toggle => {
                actions::send_command(app, "{\"command\": [\"cycle\", \"pause\"]}\n");
                app.is_paused = !app.is_paused;
                if let Some(mc) = &mut app.media_controller {
                    let _ = mc.set_playback_status(!app.is_paused);
                }
            }
            MediaEvent::Stop => {
                actions::stop_playback(app);
            }
            MediaEvent::Next => {
                actions::send_command(app, "{\"command\": [\"seek\", 10, \"relative\"]}\n");
            }
            MediaEvent::Previous => {
                actions::send_command(app, "{\"command\": [\"seek\", -10, \"relative\"]}\n");
            }
        }
    }

    // Check if playback process finished
    if let Some(ref mut child) = app.playback_process {
        if let Ok(Some(_)) = child.try_wait() {
            app.playback_process = None;
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
    }

    // Process IPC responses for progress tracking
    while let Ok(msg) = app.playback_res_rx.try_recv() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
            if let Some(t) = val["data"].as_f64() {
                if val["request_id"].as_u64() == Some(1) {
                    app.playback_time = t;
                } else if val["request_id"].as_u64() == Some(2) {
                    app.playback_total = t;
                }
            }
        }
    }

    // Update progress string and finishing state
    if app.playback_total > 0.0 {
        let current = yt::format_duration(app.playback_time);
        let total = yt::format_duration(app.playback_total);
        app.playback_duration_str = Some(format!("{}/{}", current, total));

        app.is_finishing = app.playback_total - app.playback_time < 2.0;
    }

    // Trigger property requests if we are playing
    if app.playback_cmd_tx.is_some() && !app.is_paused {
        actions::send_command(
            app,
            "{\"command\": [\"get_property\", \"time-pos\"], \"request_id\": 1}
",
        );
        actions::send_command(
            app,
            "{\"command\": [\"get_property\", \"duration\"], \"request_id\": 2}
",
        );
    }

    // Update terminal loading progress
    if app.terminal_loading {
        app.terminal_loading_progress += 0.02;
        if app.terminal_loading_progress > 0.95 {
            app.terminal_loading_progress = 0.95;
        }

        // Check if terminal is ready
        while let Ok(res) = app.terminal_ready_rx.try_recv() {
            match res {
                Ok(url) => {
                    app.terminal_ready_url = Some(url);
                    app.terminal_loading_progress = 1.0;
                }
                Err(e) => {
                    app.terminal_loading_error = Some(e);
                    app.terminal_loading_progress = 0.0;
                }
            }
        }
    }
}

pub fn move_selection(app: &mut App, delta: i32) {
    if app.search_results.is_empty() {
        app.selected_result_index = None;
        return;
    }

    let len = if !app.search_results.is_empty() && !app.is_url_mode {
        app.search_results.len() + 1 
    } else {
        app.search_results.len()
    };
    let current = app.selected_result_index.unwrap_or(0);

    let new_index = if delta > 0 {
        (current + (delta as usize)).min(len - 1)
    } else {
        current.saturating_sub(delta.abs() as usize)
    };

    app.selected_result_index = Some(new_index);

    if new_index < app.search_results.len() {
        request_image_for_selection(app);
    }
}

pub fn request_image_for_selection(app: &mut App) {
    if let Some(idx) = app.selected_result_index {
        if let Some(video) = app.search_results.get(idx) {
            if !app.image_cache.contains(&video.id) {
                if let Some(url) = &video.thumbnail_url {
                    let _ = app.image_tx.send((video.id.clone(), url.clone()));
                }
            }
        }
    }
}