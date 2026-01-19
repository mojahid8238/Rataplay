use crate::model::Video;
use crate::model::local::LocalFile;
use crate::sys::{image as sys_image, yt, local};
use crate::sys::media::{MediaController, MediaEvent};
use image::DynamicImage;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::{
    AppAction, AppState, DownloadControl, DownloadManager, InputMode
};

pub struct App {
    pub running: bool,
    pub input_mode: InputMode,
    pub state: AppState,
    pub previous_app_state: AppState,
    // Search
    pub search_query: String,
    pub cursor_position: usize,
    // Results
    pub search_results: Vec<Video>,
    pub selected_result_index: Option<usize>,
    // Async Communication
    pub search_tx: UnboundedSender<(String, u32, u32, usize)>, // query, start, end, search_id
    pub result_rx: UnboundedReceiver<Result<(yt::SearchResult, usize), String>>,
    // Search Progress
    pub search_progress: Option<f32>,
    pub search_offset: u32,
    pub is_searching: bool,
    pub current_search_id: usize,
    // If the current search was a direct URL
    pub is_url_mode: bool,
    // Messages/Status
    pub status_message: Option<String>,
    // Actions
    pub pending_action: Option<(AppAction, String, String)>, // (Action, URL, Title)
    // Images
    pub image_tx: UnboundedSender<(String, String)>, // (ID, URL)
    pub image_rx: UnboundedReceiver<(String, DynamicImage)>,
    pub image_cache: LruCache<String, DynamicImage>,
    // Download / Formats
    pub format_tx: UnboundedSender<String>, // URL
    pub format_rx: UnboundedReceiver<Result<Vec<crate::model::VideoFormat>, String>>,
    pub formats: Vec<crate::model::VideoFormat>,
    pub selected_format_index: Option<usize>,
    // Background Download
    pub download_manager: DownloadManager,
    pub new_download_tx: UnboundedSender<(Video, String)>, // Video, FormatID
    pub download_event_rx: UnboundedReceiver<crate::model::download::DownloadEvent>,
    pub download_control_tx: UnboundedSender<DownloadControl>,
    pub selected_download_index: Option<usize>,
    pub selected_download_indices: std::collections::HashSet<usize>,
    // Local Files
    pub local_files: Vec<LocalFile>,
    pub selected_local_file_index: Option<usize>,
    pub selected_local_file_indices: std::collections::HashSet<usize>,
    
    // Playback
    pub playback_process: Option<tokio::process::Child>,
    pub playback_cmd_tx: Option<UnboundedSender<String>>,
    pub playback_res_rx: UnboundedReceiver<String>,
    pub playback_title: Option<String>,
    pub playback_time: f64,
    pub playback_total: f64,
    pub playback_duration_str: Option<String>,
    pub is_paused: bool,
    pub is_finishing: bool,
    pub terminal_loading: bool,
    pub terminal_loading_progress: f32,
    pub terminal_ready_url: Option<String>,
    pub terminal_loading_error: Option<String>,
    pub terminal_ready_tx: UnboundedSender<Result<String, String>>,
    pub terminal_ready_rx: UnboundedReceiver<Result<String, String>>,

    // Details Resolution
    pub details_tx: UnboundedSender<Vec<String>>,
    pub details_rx: UnboundedReceiver<Result<Video, String>>,
    pub pending_resolution_ids: Vec<String>,

    // Playlist / Multi-select
    pub playlist_stack: Vec<(Video, Vec<Video>, Option<usize>)>, // (parent, children, prev_selected)
    pub selected_playlist_indices: std::collections::HashSet<usize>,
    pub show_downloads_panel: bool,
    
    // Media Controls
    pub media_controller: Option<MediaController>,
    pub media_rx: UnboundedReceiver<MediaEvent>,
}

impl App {
    pub fn new() -> Self {
        let (search_tx, mut search_rx) = mpsc::unbounded_channel::<(String, u32, u32, usize)>();
        let (result_tx, result_rx) =
            mpsc::unbounded_channel::<Result<(yt::SearchResult, usize), String>>();

        // Spawn a background task to handle search requests
        tokio::spawn(async move {
            while let Some((query, start, end, id)) = search_rx.recv().await {
                let tx = result_tx.clone();
                tokio::spawn(async move {
                    let (item_tx, mut item_rx) = mpsc::unbounded_channel();

                    let search_handle = tokio::spawn(async move {
                        if let Err(e) =
                            yt::search_videos_flat(&query, start, end, item_tx.clone()).await
                        {
                            let _ = item_tx.send(Err(e.to_string()));
                        }
                    });

                    while let Some(res) = item_rx.recv().await {
                        match res {
                            Ok(item) => {
                                let _ = tx.send(Ok((item, id)));
                            }
                            Err(e) => {
                                let _ = tx.send(Err(e));
                            }
                        }
                    }
                    let _ = search_handle.await;
                });
            }
        });

        let (image_tx, mut image_cmd_rx) = mpsc::unbounded_channel::<(String, String)>();
        let (image_res_tx, image_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some((id, url)) = image_cmd_rx.recv().await {
                if let Ok(img) = sys_image::download_image(&url).await {
                    let _ = image_res_tx.send((id, img));
                }
            }
        });

        let (format_tx, mut format_req_rx) = mpsc::unbounded_channel::<String>();
        let (format_res_tx, format_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(url) = format_req_rx.recv().await {
                match yt::get_video_formats(&url).await {
                    Ok(formats) => {
                        let _ = format_res_tx.send(Ok(formats));
                    }
                    Err(e) => {
                        let _ = format_res_tx.send(Err(e.to_string()));
                    }
                }
            }
        });

        let (new_download_tx, mut new_download_cmd_rx) = mpsc::unbounded_channel::<(Video, String)>();
        let (download_event_tx, download_event_rx) = mpsc::unbounded_channel();
        let (download_control_tx, mut download_control_rx) = mpsc::unbounded_channel::<DownloadControl>();

        use libc::{kill, SIGSTOP, SIGCONT, SIGTERM};
        use tokio::io::{AsyncBufReadExt, BufReader};
        use std::collections::HashMap;

        // Spawn a background task to handle download requests and control messages
        tokio::spawn(async move {
            // Map video_id to its PID for control (pause/resume/cancel)
            let mut active_downloads_pids: HashMap<String, u32> = HashMap::new();

            loop {
                tokio::select! {
                    // Handle new download requests
                    res = new_download_cmd_rx.recv() => {
                        if let Some((video, format_id)) = res {
                            let event_tx = download_event_tx.clone();
                            let video_id = video.id.clone();
                            let mut child = match crate::sys::download::start_download(&video, &format_id).await {
                                Ok(child) => child,
                                Err(e) => {
                                    let _ = event_tx.send(crate::model::download::DownloadEvent::Error(video_id.clone(), e.to_string()));
                                    continue;
                                }
                            };
                            let pid = child.id().expect("Failed to get child process ID");
                            let _ = event_tx.send(crate::model::download::DownloadEvent::Started(video_id.clone(), pid));

                            active_downloads_pids.insert(video_id.clone(), pid);

                            // Spawn a separate task to monitor this specific download's stdout/stderr and status
                            let monitor_event_tx = event_tx.clone();
                            tokio::spawn(async move {
                                let stdout = child
                                    .stdout
                                    .take()
                                    .expect("child did not have a handle to stdout");
                                let stderr = child
                                    .stderr
                                    .take()
                                    .expect("child did not have a handle to stderr");

                                let mut stdout_reader = BufReader::new(stdout).lines();
                                let mut stderr_reader = BufReader::new(stderr).lines();

                                let mut last_progress_update = Instant::now();
                                let min_update_interval = Duration::from_millis(500);

                                loop {
                                    tokio::select! {
                                        Ok(Some(line)) = stdout_reader.next_line() => {
                                            if let Some((progress, total_size, speed, eta)) =
                                                crate::sys::download::parse_progress(&line)
                                            {
                                                if last_progress_update.elapsed() >= min_update_interval {
                                                    let _ = monitor_event_tx.send(crate::model::download::DownloadEvent::Update(
                                                        video_id.clone(),
                                                        progress,
                                                        speed,
                                                        eta,
                                                        total_size,
                                                    ));
                                                    last_progress_update = Instant::now();
                                                }
                                            } else {
                                                // eprintln!("yt-dlp stdout for {}: {}", video_id, line);
                                            }
                                        }
                                        Ok(Some(_line)) = stderr_reader.next_line() => {
                                            // Handle stderr messages if needed, potentially errors or warnings
                                            // eprintln!("yt-dlp stderr for {}: {}", video_id, line);
                                        }
                                        status = child.wait() => {
                                            match status {
                                                Ok(exit_status) => {
                                                    if exit_status.success() {
                                                        let _ = monitor_event_tx.send(crate::model::download::DownloadEvent::Finished(video_id.clone()));
                                                    } else {
                                                        let _ = monitor_event_tx.send(crate::model::download::DownloadEvent::Error(
                                                            video_id.clone(),
                                                            format!("Download failed with exit code: {:?}", exit_status.code()),
                                                        ));
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = monitor_event_tx.send(crate::model::download::DownloadEvent::Error(
                                                        video_id.clone(),
                                                        format!("Failed to wait for download process: {}", e),
                                                    ));
                                                }
                                            }
                                            break; // Download process finished
                                        }
                                        else => break, // All streams closed and child exited
                                    }
                                }
                            });
                        } else {
                            break;
                        }
                    }
                    // Handle control messages for existing downloads
                    res = download_control_rx.recv() => {
                        if let Some(control) = res {
                            match control {
                                DownloadControl::Pause(id) => {
                                    if let Some(&pid) = active_downloads_pids.get(&id) {
                                        let _ = unsafe { kill(pid as i32, SIGSTOP) };
                                        let _ = download_event_tx.send(crate::model::download::DownloadEvent::Pause(id));
                                    }
                                }
                                DownloadControl::Resume(id) => {
                                    if let Some(&pid) = active_downloads_pids.get(&id) {
                                        let _ = unsafe { kill(pid as i32, SIGCONT) };
                                        let _ = download_event_tx.send(crate::model::download::DownloadEvent::Resume(id));
                                    }
                                }
                                DownloadControl::Cancel(id) => {
                                    if let Some(pid) = active_downloads_pids.remove(&id) {
                                        let _ = unsafe { kill(pid as i32, SIGTERM) };
                                        // The monitor task for this child will eventually send the Error/Finished event
                                        // and clean up its own child process.
                                        let _ = download_event_tx.send(crate::model::download::DownloadEvent::Canceled(id));
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        });


        let (_, playback_res_rx) = mpsc::unbounded_channel();
        let (terminal_ready_tx, terminal_ready_rx) =
            mpsc::unbounded_channel::<Result<String, String>>();

        let (details_tx, mut details_req_rx) = mpsc::unbounded_channel::<Vec<String>>();
        let (details_res_tx, details_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(ids) = details_req_rx.recv().await {
                let res_tx = details_res_tx.clone();
                if let Err(e) = yt::resolve_video_details(ids, res_tx.clone()).await {
                    let _ = res_tx.send(Err(e.to_string()));
                }
            }
        });

        // Scan local files initially
        let local_files = local::scan_local_files();
        let mut download_manager = DownloadManager::new();
        
        // Scan for incomplete downloads to resume
        let incomplete = local::scan_incomplete_downloads();
        for (id, title, url) in incomplete {
            if !download_manager.tasks.contains_key(&id) {
                let mut video = Video::default();
                video.id = id.clone();
                video.title = title.clone();
                video.url = url;
                
                let mut task = crate::model::download::DownloadTask::new(video, "best".to_string());
                task.status = crate::model::download::DownloadStatus::Canceled; // Set to canceled so it shows up as restorable
                download_manager.tasks.insert(id.clone(), task);
                download_manager.task_order.push(id);
            }
        }

        let (media_tx, media_rx) = mpsc::unbounded_channel();
        let media_controller = MediaController::init(media_tx).ok();

        Self {
            running: true,
            input_mode: InputMode::Editing,
            state: AppState::Search,
            previous_app_state: AppState::Search,
            search_query: String::new(),
            cursor_position: 0,
            search_results: Vec::new(),
            selected_result_index: None,
            search_tx,
            result_rx,
            search_progress: None,
            search_offset: 1,
            is_searching: false,
            current_search_id: 0,
            is_url_mode: false,
            status_message: None,
            pending_action: None,
            image_tx,
            image_rx,
            image_cache: LruCache::new(NonZeroUsize::new(50).unwrap()),
            format_tx,
            format_rx,
            formats: Vec::new(),
            selected_format_index: None,
            download_manager,
            new_download_tx,
            download_event_rx,
            download_control_tx,
            selected_download_index: None,
            selected_download_indices: std::collections::HashSet::new(),
            local_files,
            selected_local_file_index: None,
            selected_local_file_indices: std::collections::HashSet::new(),
            playback_process: None,
            playback_cmd_tx: None,
            playback_res_rx,
            playback_title: None,
            playback_time: 0.0,
            playback_total: 0.0,
            playback_duration_str: None,
            is_paused: false,
            is_finishing: false,
            terminal_loading: false,
            terminal_loading_progress: 0.0,
            terminal_loading_error: None,
            terminal_ready_url: None,
            terminal_ready_tx,
            terminal_ready_rx,
            details_tx,
            details_rx,
            pending_resolution_ids: Vec::new(),
            playlist_stack: Vec::new(),
            selected_playlist_indices: std::collections::HashSet::new(),
            show_downloads_panel: false,
            media_controller,
            media_rx,
        }
    }
}
