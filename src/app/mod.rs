use crate::model::download::{DownloadEvent, DownloadTask};
use crate::model::Video;
use crate::model::local::LocalFile;
use crate::sys::{image as sys_image, yt, local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use image::DynamicImage;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;

use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Loading, // Added Loading state
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppAction {
    WatchExternal,
    WatchInTerminal,
    ListenAudio,
    Download,
    DownloadPlaylist,
    DownloadSelected,
    ViewPlaylist,
    // Local Actions
    PlayLocalExternal,
    PlayLocalTerminal, // Placeholder, maybe same as external or specialized
    PlayLocalAudio,    // Placeholder
    DeleteLocalFile,
    DeleteSelectedLocalFiles,
    CleanupLocalGarbage,
    CancelSelectedDownloads,
    CancelDownload,
    ResumeDownload,
    ResumeSelectedDownloads,
}
pub struct Action {
    pub key: KeyCode,
    pub name: &'static str,
    pub action: AppAction,
}
impl Action {
    pub fn new(key: KeyCode, name: &'static str, action: AppAction) -> Self {
        Self { key, name, action }
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppState {
    Search,
    Results,
    ActionMenu,
    FormatSelection,
    Downloads,
}

#[derive(Debug)]
pub enum DownloadControl {
    Pause(String), // id
    Resume(String), // id
    Cancel(String), // id
}
pub struct DownloadManager {
    pub tasks: HashMap<String, DownloadTask>,
    pub task_order: Vec<String>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            task_order: Vec::new(),
        }
    }

    pub fn add_task(&mut self, video: &Video, format_id: &str) {
        let task = DownloadTask::new(video.clone(), format_id.to_string());
        self.tasks.insert(video.id.clone(), task);
        self.task_order.push(video.id.clone());
    }
}
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
    pub download_event_rx: UnboundedReceiver<DownloadEvent>,
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
                                    let _ = event_tx.send(DownloadEvent::Error(video_id.clone(), e.to_string()));
                                    continue;
                                }
                            };
                            let pid = child.id().expect("Failed to get child process ID");
                            let _ = event_tx.send(DownloadEvent::Started(video_id.clone(), pid));

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
                                                    let _ = monitor_event_tx.send(DownloadEvent::Update(
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
                                                        let _ = monitor_event_tx.send(DownloadEvent::Finished(video_id.clone()));
                                                    } else {
                                                        let _ = monitor_event_tx.send(DownloadEvent::Error(
                                                            video_id.clone(),
                                                            format!("Download failed with exit code: {:?}", exit_status.code()),
                                                        ));
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = monitor_event_tx.send(DownloadEvent::Error(
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
                                        let _ = download_event_tx.send(DownloadEvent::Pause(id));
                                    }
                                }
                                DownloadControl::Resume(id) => {
                                    if let Some(&pid) = active_downloads_pids.get(&id) {
                                        let _ = unsafe { kill(pid as i32, SIGCONT) };
                                        let _ = download_event_tx.send(DownloadEvent::Resume(id));
                                    }
                                }
                                DownloadControl::Cancel(id) => {
                                    if let Some(pid) = active_downloads_pids.remove(&id) {
                                        let _ = unsafe { kill(pid as i32, SIGTERM) };
                                        // The monitor task for this child will eventually send the Error/Finished event
                                        // and clean up its own child process.
                                        let _ = download_event_tx.send(DownloadEvent::Canceled(id));
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
            download_manager: DownloadManager::new(),
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
        }
    }
    
    pub fn refresh_local_files(&mut self) {
        self.local_files = local::scan_local_files();
        if !self.local_files.is_empty() {
             if self.selected_local_file_index.is_none() {
                 self.selected_local_file_index = Some(0);
             } else if let Some(idx) = self.selected_local_file_index {
                 if idx >= self.local_files.len() {
                     self.selected_local_file_index = Some(self.local_files.len().saturating_sub(1));
                 }
             }
        } else {
            self.selected_local_file_index = None;
        }
    }

    pub fn get_available_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();

        let current_context = if self.state == AppState::ActionMenu {
            self.previous_app_state
        } else {
            self.state
        };

        // Local Actions (High Priority if in Downloads view)
        if current_context == AppState::Downloads {
            // Actions for Local Files
            if let Some(idx) = self.selected_local_file_index {
                 if let Some(file) = self.local_files.get(idx) {
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
            if let Some(idx) = self.selected_download_index {
                if let Some(task_id) = self.download_manager.task_order.get(idx) {
                    if let Some(task) = self.download_manager.tasks.get(task_id) {
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

            if !self.selected_download_indices.is_empty() {
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

             if !self.selected_local_file_indices.is_empty() {
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


        if let Some(idx) = self.selected_result_index {
            if let Some(video) = self.search_results.get(idx) {
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

        if !self.selected_playlist_indices.is_empty() {
            actions.push(Action::new(
                KeyCode::Char('s'),
                "Download Selected",
                AppAction::DownloadSelected,
            ));
        }

        if !self.playlist_stack.is_empty() {
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

    pub fn on_tick(&mut self) {
        // Periodic refresh of local files if download view is active
        if self.show_downloads_panel && self.state == AppState::Downloads {
             // Only refresh occasionally to save resources? 
             // Or just check if task count changed? 
             // For now, let's refresh every tick is too much. 
             // We'll rely on events or manual refresh, OR refresh if download manager updates.
             // But scanning FS is heavy. 
             // Let's Refresh only when tasks update or via specific trigger?
             // Actually, `on_tick` is called frequently. Let's not scan here.
             // We can scan when entering the view.
        }

        // check for search results
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                Ok((item, id)) => {
                    if id != self.current_search_id {
                        continue;
                    }
                    match item {
                        yt::SearchResult::Video(video) => {
                            // Trigger image download if thumbnail exists, even if partial
                            if let Some(url) = &video.thumbnail_url {
                                if !self.image_cache.contains(&video.id) {
                                    let _ = self.image_tx.send((video.id.clone(), url.clone()));
                                }
                            }

                            self.search_results.push(video);
                            if self.selected_result_index.is_none() {
                                self.selected_result_index = Some(0);
                                self.request_image_for_selection();
                            }
                            if self.state == AppState::Search {
                                self.state = AppState::Results;
                            }
                        }
                        yt::SearchResult::Progress(progress) => {
                            self.search_progress = Some(progress);
                            if progress >= 1.0 {
                                self.is_searching = false;
                                self.search_progress = None;
                                if self.search_results.is_empty() {
                                    self.status_message = Some("No results found.".to_string());
                                } else {
                                    self.status_message = Some("Results updated.".to_string());
                                }

                                // Flush pending resolutions
                                if !self.pending_resolution_ids.is_empty() {
                                    let items: Vec<String> =
                                        self.pending_resolution_ids.drain(..).collect();
                                    let _ = self.details_tx.send(items);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    self.is_searching = false;
                    self.status_message = Some(format!("Error: {}", e));
                    self.search_progress = None;
                }
            }
        }

        // Resolve details for the currently selected item if it's partial
        if let Some(idx) = self.selected_result_index {
            if let Some(video) = self.search_results.get(idx) {
                if video.is_partial && video.video_type == crate::model::VideoType::Video {
                    if !self.pending_resolution_ids.contains(&video.url) {
                        self.pending_resolution_ids.push(video.url.clone());
                        let _ = self.details_tx.send(vec![video.url.clone()]);
                    }
                }
            }
        }

        // Apply resolved details
        while let Ok(res) = self.details_rx.try_recv() {
            match res {
                Ok(v) => {
                    let url = v.url.clone();
                    // Find and replace in search_results
                    if let Some(existing) = self.search_results.iter_mut().find(|x| x.id == v.id) {
                        *existing = v;
                    }
                    // Remove from pending
                    self.pending_resolution_ids.retain(|x| x != &url);
                    
                    // Trigger image request for selection again if needed
                    self.request_image_for_selection();
                }
                Err(e) => {
                    self.status_message = Some(format!("Details error: {}", e));
                }
            }
        }

        // check for images AND trigger new downloads if necessary
        while let Ok((id, img)) = self.image_rx.try_recv() {
            self.image_cache.put(id, img);
        }

        // Trigger download for selected item if needed
        if let Some(idx) = self.selected_result_index {
            if let Some(video) = self.search_results.get(idx) {
                if !self.image_cache.contains(&video.id) {
                    if !self.image_cache.contains(&video.id) {
                        // Logic handled in request_image_for_selection
                    }
                }
            }
        }
        // Check for formats
        // Check for formats
        if let Ok(res) = self.format_rx.try_recv() {
            match res {
                Ok(formats) => {
                    self.formats = formats;
                    if !self.formats.is_empty() {
                        self.selected_format_index = Some(0);
                    }
                    // If we were waiting (Loading), switch to FormatSelection
                    if self.input_mode == InputMode::Loading {
                        self.input_mode = InputMode::Normal;
                        self.state = AppState::FormatSelection;
                    }
                }
                Err(e) => {
                    self.input_mode = InputMode::Normal;
                    self.status_message = Some(format!("Error fetching formats: {}", e));
                }
            }
        }

        // Check for download events
        while let Ok(event) = self.download_event_rx.try_recv() {
            match event {
                DownloadEvent::Update(id, progress, speed, eta, total_size) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        task.status = crate::model::download::DownloadStatus::Downloading;
                        task.progress = progress;
                        task.speed = speed;
                        task.eta = eta;
                        task.total_size = total_size;
                    }
                }
                DownloadEvent::Finished(id) => {
                    self.download_manager.tasks.remove(&id);
                    self.download_manager.task_order.retain(|x| x != &id);
                    // Clear indices because order changed
                    self.selected_download_indices.clear();
                    self.selected_download_index = None;
                    
                    // Refresh local files when a download finishes
                     self.refresh_local_files();
                }
                DownloadEvent::Error(id, error) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        // Only set to Error if not already Canceled by user
                        if task.status != crate::model::download::DownloadStatus::Canceled {
                            task.status = crate::model::download::DownloadStatus::Error(error);
                        }
                    }
                }
                DownloadEvent::Started(id, pid) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        task.pid = Some(pid);
                    }
                    // Refresh local files to see .part file
                    self.refresh_local_files();
                }
                DownloadEvent::Pause(id) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        task.status = crate::model::download::DownloadStatus::Paused;
                    }
                }
                DownloadEvent::Resume(id) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        task.status = crate::model::download::DownloadStatus::Downloading;
                    }
                }
                DownloadEvent::Canceled(id) => {
                    if let Some(task) = self.download_manager.tasks.get_mut(&id) {
                        task.status = crate::model::download::DownloadStatus::Canceled;
                        task.speed = String::new();
                        task.eta = String::new();
                    }
                    // Refresh to remove part file if deleted (though canceling might not auto-delete in all cases, mostly it stops)
                    self.refresh_local_files();
                }

            }
        }

        // Check if playback process finished
        if let Some(ref mut child) = self.playback_process {
            if let Ok(Some(_)) = child.try_wait() {
                self.playback_process = None;
                self.playback_cmd_tx = None;
                self.playback_title = None;
                self.playback_time = 0.0;
                self.playback_total = 0.0;
                self.playback_duration_str = None;
                self.is_paused = false;
                self.is_finishing = false;
            }
        }

        // Process IPC responses for progress tracking
        while let Ok(msg) = self.playback_res_rx.try_recv() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&msg) {
                if let Some(t) = val["data"].as_f64() {
                    if val["request_id"].as_u64() == Some(1) {
                        self.playback_time = t;
                    } else if val["request_id"].as_u64() == Some(2) {
                        self.playback_total = t;
                    }
                }
            }
        }

        // Update progress string and finishing state
        if self.playback_total > 0.0 {
            let current = yt::format_duration(self.playback_time);
            let total = yt::format_duration(self.playback_total);
            self.playback_duration_str = Some(format!("{}/{}", current, total));

            // Mark as finishing if within 2 seconds of end
            self.is_finishing = self.playback_total - self.playback_time < 2.0;
        }

        // Trigger property requests if we are playing
        if self.playback_cmd_tx.is_some() && !self.is_paused {
            // Request time and duration
            self.send_command(
                "{\"command\": [\"get_property\", \"time-pos\"], \"request_id\": 1}\n",
            );
            self.send_command(
                "{\"command\": [\"get_property\", \"duration\"], \"request_id\": 2}\n",
            );
        }

        // Update terminal loading progress
        if self.terminal_loading {
            self.terminal_loading_progress += 0.02;
            if self.terminal_loading_progress > 0.95 {
                self.terminal_loading_progress = 0.95;
            }

            // Check if terminal is ready
            while let Ok(res) = self.terminal_ready_rx.try_recv() {
                match res {
                    Ok(url) => {
                        self.terminal_ready_url = Some(url);
                        self.terminal_loading_progress = 1.0;
                    }
                    Err(e) => {
                        self.terminal_loading_error = Some(e);
                        self.terminal_loading_progress = 0.0;
                        // Don't set terminal_loading to false yet, let TUI show error
                        // Actually, we should probably auto-exit loading mode after a few seconds or on Esc
                    }
                }
            }
        }
    }

    pub fn start_terminal_loading(&mut self, url: String, _title: String) {
        self.terminal_loading = true;
        self.terminal_loading_progress = 0.0;
        self.terminal_loading_error = None;
        self.terminal_ready_url = None;
        let tx = self.terminal_ready_tx.clone();

        tokio::spawn(async move {
            // Stage 1: Fetch direct URL and User-Agent
            let mut cmd = tokio::process::Command::new("yt-dlp");
            cmd.arg("--user-agent");
            let ua = match cmd.output().await {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout).trim().to_string()
                }
                _ => "Mozilla/5.0".to_string(), // Fallback
            };

            match yt::get_best_stream_url(&url).await {
                Ok(direct_url) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    // Send as a single string joined by a special marker if needed,
                    // but we'll use a hack: "URL|UA"
                    let _ = tx.send(Ok(format!("{}|{}", direct_url, ua)));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        });
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        let code = match self.input_mode {
            InputMode::Editing => key.code,
            _ => match key.code {
                KeyCode::Char(c) => KeyCode::Char(c.to_lowercase().next().unwrap_or(c)),
                _ => key.code,
            },
        };

        match self.input_mode {
            InputMode::Normal => {
                match self.state {
                    AppState::FormatSelection => match code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.state = AppState::ActionMenu;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let Some(idx) = self.selected_format_index {
                                if idx > 0 {
                                    self.selected_format_index = Some(idx - 1);
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let Some(idx) = self.selected_format_index {
                                if idx < self.formats.len().saturating_sub(1) {
                                    self.selected_format_index = Some(idx + 1);
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = self.selected_format_index {
                                if let Some(fmt) = self.formats.get(idx) {
                                    if let Some(res_idx) = self.selected_result_index {
                                        if let Some(video) = self.search_results.get(res_idx) {
                                            // Add to manager and start download
                                            self.download_manager.add_task(video, &fmt.format_id);
                                            let _ = self
                                                .new_download_tx
                                                .send((video.clone(), fmt.format_id.clone()));
                                            self.state = AppState::Results;
                                            self.status_message =
                                                Some("Download started...".to_string());
                                            return;
                                        }
                                    }
                                }
                            }
                            self.state = AppState::Results; // Return to list after download start
                        }
                        _ => {}
                    },
                    AppState::Downloads => match code {
                        KeyCode::Tab | KeyCode::Esc => {
                            self.state = AppState::Results;
                        }
                        KeyCode::Char('q') => {
                            self.running = false;
                        }
                        KeyCode::Char('b') => {
                            self.show_downloads_panel = false;
                            self.state = self.previous_app_state;
                        }
                        KeyCode::Char('/') | KeyCode::Char('s') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            // If focus is on Local Files
                            if let Some(idx) = self.selected_local_file_index {
                                if idx > 0 {
                                    self.selected_local_file_index = Some(idx - 1);
                                } else if !self.download_manager.task_order.is_empty() {
                                    // Jump up to Active Downloads
                                    self.selected_local_file_index = None;
                                    self.selected_download_index = Some(self.download_manager.task_order.len() - 1);
                                }
                            } else if let Some(idx) = self.selected_download_index {
                                // Focus is on Active Downloads
                                if idx > 0 {
                                    self.selected_download_index = Some(idx - 1);
                                }
                            } else {
                                // Default initialization if needed
                                if !self.local_files.is_empty() {
                                    self.selected_local_file_index = Some(0);
                                } else if !self.download_manager.task_order.is_empty() {
                                    self.selected_download_index = Some(0);
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            // If focus is on Active Downloads
                            if let Some(idx) = self.selected_download_index {
                                if idx < self.download_manager.task_order.len() - 1 {
                                    self.selected_download_index = Some(idx + 1);
                                } else if !self.local_files.is_empty() {
                                    // Jump down to Local Files
                                    self.selected_download_index = None;
                                    self.selected_local_file_index = Some(0);
                                }
                            } else if let Some(idx) = self.selected_local_file_index {
                                // Focus is on Local Files
                                if idx < self.local_files.len().saturating_sub(1) {
                                    self.selected_local_file_index = Some(idx + 1);
                                }
                            } else {
                                // Default initialization
                                if !self.download_manager.task_order.is_empty() {
                                    self.selected_download_index = Some(0);
                                } else if !self.local_files.is_empty() {
                                    self.selected_local_file_index = Some(0);
                                }
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(idx) = self.selected_download_index {
                                if self.selected_download_indices.contains(&idx) {
                                    self.selected_download_indices.remove(&idx);
                                } else {
                                    self.selected_download_indices.insert(idx);
                                }
                            } else if let Some(idx) = self.selected_local_file_index {
                                if self.selected_local_file_indices.contains(&idx) {
                                    self.selected_local_file_indices.remove(&idx);
                                } else {
                                    self.selected_local_file_indices.insert(idx);
                                }
                            }
                        }
                        KeyCode::Enter => {
                             if self.selected_local_file_index.is_some() || self.selected_download_index.is_some() {
                                 self.previous_app_state = self.state;
                                 self.state = AppState::ActionMenu;
                             }
                        }
                        // Handle direct actions for Active Downloads shortcuts if desired (p=pause, x=cancel)
                         KeyCode::Char('p') => {
                            let mut handled = false;
                            if let Some(idx) = self.selected_download_index {
                                 if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                      if let Some(task) = self.download_manager.tasks.get(task_id) {
                                          match task.status {
                                              crate::model::download::DownloadStatus::Downloading => {
                                                  let _ = self.download_control_tx.send(DownloadControl::Pause(task_id.clone()));
                                              }
                                              crate::model::download::DownloadStatus::Paused => {
                                                  let _ = self.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                              }
                                              crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                                  // Restart canceled or failed download
                                                  let video = task.video.clone();
                                                  let format_id = task.format_id.clone();
                                                  let _ = self.new_download_tx.send((video, format_id));
                                                  
                                                  if let Some(t) = self.download_manager.tasks.get_mut(task_id) {
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
                                self.toggle_pause();
                            }
                        }
                        KeyCode::Char('x') => {
                             let mut handled = false;
                             if let Some(idx) = self.selected_download_index {
                                 if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                     let _ = self.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                     handled = true;
                                 }
                             }
                             if !handled {
                                 self.stop_playback();
                             }
                        }
                        KeyCode::Left => {
                            self.seek(-5);
                        }
                        KeyCode::Right => {
                            self.seek(5);
                        }
                        KeyCode::Char('[') => {
                            self.seek(-30);
                        }
                        KeyCode::Char(']') => {
                            self.seek(30);
                        }
                        _ => {} 
                    },
                    AppState::ActionMenu => {
                        if code == KeyCode::Esc || code == KeyCode::Char('q') {
                            self.state = self.previous_app_state;
                            return;
                        }

                        if let Some(action) =
                            self.get_available_actions().iter().find(|a| a.key == code)
                        {
                            // HANDLE LOCAL ACTIONS
                            match action.action {
                                AppAction::PlayLocalExternal => {
                                     if let Some(idx) = self.selected_local_file_index {
                                         if let Some((path, name)) = self.local_files.get(idx).map(|f| (f.path.to_string_lossy().to_string(), f.name.clone())) {
                                             self.stop_playback();
                                             self.pending_action = Some((
                                                 AppAction::WatchExternal,
                                                 path,
                                                 name
                                             ));
                                             self.state = self.previous_app_state;
                                         }
                                     }
                                }
                                AppAction::PlayLocalTerminal => {
                                     if let Some(idx) = self.selected_local_file_index {
                                         if let Some(file) = self.local_files.get(idx) {
                                             let path = file.path.to_string_lossy().to_string();
                                             let name = file.name.clone();
                                             let is_audio = file.is_audio();
                                             
                                             self.stop_playback();
                                             
                                             if is_audio {
                                                 // Redirect to background playback for better experience
                                                 self.pending_action = Some((
                                                     AppAction::ListenAudio,
                                                     path,
                                                     name
                                                 ));
                                             } else {
                                                 // For local video files, the path is the URL
                                                 self.terminal_ready_url = Some(path);
                                             }
                                             self.state = self.previous_app_state;
                                         }
                                     }
                                }
                                AppAction::PlayLocalAudio => {
                                     if let Some(idx) = self.selected_local_file_index {
                                         if let Some((path, name)) = self.local_files.get(idx).map(|f| (f.path.to_string_lossy().to_string(), f.name.clone())) {
                                             self.stop_playback();
                                             self.pending_action = Some((
                                                 AppAction::ListenAudio,
                                                 path,
                                                 name
                                             ));
                                             self.state = self.previous_app_state;
                                         }
                                     }
                                }
                                AppAction::DeleteLocalFile => {
                                     if let Some(idx) = self.selected_local_file_index {
                                         if let Some(file) = self.local_files.get(idx) {
                                             if let Err(e) = local::delete_file(&file.path) {
                                                 self.status_message = Some(format!("Error deleting: {}", e));
                                             } else {
                                                 self.status_message = Some("File deleted.".to_string());
                                                 self.refresh_local_files();
                                             }
                                         }
                                     }
                                     self.state = self.previous_app_state;
                                }
                                AppAction::DeleteSelectedLocalFiles => {
                                     let indices: Vec<usize> = self.selected_local_file_indices.iter().cloned().collect();
                                     for &idx in &indices {
                                         if let Some(file) = self.local_files.get(idx) {
                                             let _ = local::delete_file(&file.path);
                                         }
                                     }
                                     
                                     // If focused file was deleted, reset focus
                                     if let Some(idx) = self.selected_local_file_index {
                                         if indices.contains(&idx) {
                                             self.selected_local_file_index = None;
                                         }
                                     }
                                     
                                     self.selected_local_file_indices.clear();
                                     self.refresh_local_files();
                                     self.status_message = Some(format!("Deleted {} files.", indices.len()));
                                     self.state = self.previous_app_state;
                                }
                                AppAction::ResumeDownload => {
                                     if let Some(idx) = self.selected_download_index {
                                         if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                             if let Some(task) = self.download_manager.tasks.get(task_id) {
                                                 match task.status {
                                                     crate::model::download::DownloadStatus::Downloading => {
                                                         let _ = self.download_control_tx.send(DownloadControl::Pause(task_id.clone()));
                                                     }
                                                     crate::model::download::DownloadStatus::Paused => {
                                                         let _ = self.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                                     }
                                                     crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                                         let video = task.video.clone();
                                                         let format_id = task.format_id.clone();
                                                         let _ = self.new_download_tx.send((video, format_id));
                                                         if let Some(t) = self.download_manager.tasks.get_mut(task_id) {
                                                             t.status = crate::model::download::DownloadStatus::Pending;
                                                         }
                                                     }
                                                     _ => {}
                                                 }
                                             }
                                         }
                                     }
                                     self.state = self.previous_app_state;
                                }
                                AppAction::ResumeSelectedDownloads => {
                                     let indices: Vec<usize> = self.selected_download_indices.iter().cloned().collect();
                                     for idx in indices {
                                         if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                             if let Some(task) = self.download_manager.tasks.get(task_id) {
                                                 match task.status {
                                                     crate::model::download::DownloadStatus::Paused => {
                                                         let _ = self.download_control_tx.send(DownloadControl::Resume(task_id.clone()));
                                                     }
                                                     crate::model::download::DownloadStatus::Canceled | crate::model::download::DownloadStatus::Error(_) => {
                                                         let video = task.video.clone();
                                                         let format_id = task.format_id.clone();
                                                         let _ = self.new_download_tx.send((video, format_id));
                                                         if let Some(t) = self.download_manager.tasks.get_mut(task_id) {
                                                             t.status = crate::model::download::DownloadStatus::Pending;
                                                         }
                                                     }
                                                     _ => {}
                                                 }
                                             }
                                         }
                                     }
                                     self.selected_download_indices.clear();
                                     self.state = self.previous_app_state;
                                }
                                AppAction::CancelDownload => {
                                     if let Some(idx) = self.selected_download_index {
                                         if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                             let _ = self.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                         }
                                     }
                                     self.state = self.previous_app_state;
                                }
                                AppAction::CancelSelectedDownloads => {
                                     let indices: Vec<usize> = self.selected_download_indices.iter().cloned().collect();
                                     for idx in indices {
                                         if let Some(task_id) = self.download_manager.task_order.get(idx) {
                                             let _ = self.download_control_tx.send(DownloadControl::Cancel(task_id.clone()));
                                         }
                                     }
                                     self.selected_download_indices.clear();
                                     self.state = self.previous_app_state;
                                }
                                AppAction::CleanupLocalGarbage => {
                                    match local::cleanup_garbage() {
                                        Ok(count) => {
                                            self.status_message = Some(format!("Cleaned {} garbage files.", count));
                                            self.refresh_local_files();
                                        }
                                        Err(e) => {
                                            self.status_message = Some(format!("Error cleanup: {}", e));
                                        }
                                    }
                                    self.state = self.previous_app_state;
                                }
                                // EXISTING ACTIONS
                                _ => {
                                     // ... Existing action handling ... 
                                     if let Some(idx) = self.selected_result_index {
                                        if let Some(video) = self.search_results.get(idx) {
                                            let url = video.url.clone();
                                            let title = video.title.clone();
                                            match action.action {
                                                AppAction::ViewPlaylist => {
                                                    self.status_message =
                                                        Some("Attempting to view playlist...".to_string());
                                                    // Drill down into playlist
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
                                                    let children = std::mem::take(&mut self.search_results);
                                                    self.playlist_stack.push((
                                                        parent,
                                                        children,
                                                        self.selected_result_index,
                                                    ));
                                                    self.selected_playlist_indices.clear();
                                                    self.search_results.clear(); // Clear existing results before loading new ones
                                                    self.selected_result_index = Some(0); // Reset selection for the new playlist view

                                                    self.is_searching = true;
                                                    self.search_progress = Some(0.0);
                                                    self.current_search_id += 1;
                                                    let _ = self.search_tx.send((
                                                        query,
                                                        1,
                                                        100, // Fetch more for playlists
                                                        self.current_search_id,
                                                    ));
                                                    self.status_message =
                                                        Some(format!("Loading playlist: {}...", title));
                                                    self.state = AppState::Results;
                                                    return;
                                                }
                                                AppAction::Download => {
                                                    // Trigger Format Fetch
                                                    let _ = self.format_tx.send(url);
                                                    self.input_mode = InputMode::Loading;
                                                    self.status_message =
                                                        Some("Fetching formats...".to_string());
                                                }
                                                AppAction::WatchInTerminal => {
                                                    self.stop_playback();
                                                    self.start_terminal_loading(url, title);
                                                    self.state = self.previous_app_state;
                                                }
                                                AppAction::DownloadSelected => {
                                                     let selected_videos: Vec<Video> = self
                                                        .selected_playlist_indices
                                                        .iter()
                                                        .filter_map(|&idx| self.search_results.get(idx).cloned())
                                                        .collect();


                                                    if selected_videos.is_empty() {
                                                        self.status_message =
                                                            Some("No videos selected.".to_string());
                                                    } else {
                                                        for video in selected_videos {
                                                             self.download_manager.add_task(&video, "best");
                                                            let _ = self
                                                                .new_download_tx
                                                                .send((video, "best".to_string()));
                                                        }
                                                        self.status_message =
                                                            Some("Starting downloads...".to_string());
                                                        self.state = self.previous_app_state;
                                                    }
                                                }
                                                AppAction::DownloadPlaylist => {
                                                    if let Some(_parent_url) = &video.parent_playlist_url {
                                                        self.status_message = Some("Playlist download from this context is not fully implemented. Downloading current view.".to_string());
                                                    }
                                                    let videos: Vec<Video> = self
                                                        .search_results
                                                        .iter().cloned()
                                                        .collect();
                                                    for video in videos {
                                                                                                           self.download_manager.add_task(&video, "best");                                                let _ = self
                                                            .new_download_tx
                                                            .send((video, "best".to_string()));
                                                    }
                                                    self.status_message =
                                                        Some("Starting playlist download...".to_string());

                                                    self.state = self.previous_app_state;
                                                }
                                                _ => {
                                                    self.pending_action = Some((action.action, url, title));
                                                    self.state = self.previous_app_state;
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
                            self.running = false;
                        }
                        KeyCode::Tab => {
                            if self.show_downloads_panel {
                                self.state = AppState::Downloads;
                            }
                        }
                        KeyCode::Char('d') => {
                            if self.show_downloads_panel {
                                self.show_downloads_panel = false;
                                self.state = self.previous_app_state;
                            } else {
                                self.show_downloads_panel = true;
                                self.previous_app_state = self.state;
                                self.state = AppState::Downloads;
                                self.refresh_local_files(); // Refresh when opening
                                
                                // Prioritize active downloads focus if any exist
                                if !self.download_manager.task_order.is_empty() {
                                    self.selected_download_index = Some(0);
                                    self.selected_local_file_index = None;
                                } else if !self.local_files.is_empty() {
                                    self.selected_download_index = None;
                                    self.selected_local_file_index = Some(0);
                                }
                            }
                        }
                        KeyCode::Char('/') | KeyCode::Char('s') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            self.move_selection(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            self.move_selection(-1);
                        }
                        KeyCode::Enter => {
                            if let Some(idx) = self.selected_result_index {
                                if idx < self.search_results.len() {
                                    self.previous_app_state = self.state;
                                    self.state = AppState::ActionMenu;
                                } else if !self.is_url_mode {
                                    self.load_more();
                                }
                            }
                        }
                        KeyCode::Backspace | KeyCode::Char('b') => {
                            if let Some((_parent, children, prev_idx)) = self.playlist_stack.pop() {
                                self.search_results = children;
                                self.selected_result_index = prev_idx;
                                self.selected_playlist_indices.clear();
                                self.status_message =
                                    Some("Returned to search results.".to_string());
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(idx) = self.selected_result_index {
                                if idx < self.search_results.len() {
                                    if self.selected_playlist_indices.contains(&idx) {
                                        self.selected_playlist_indices.remove(&idx);
                                    } else {
                                        self.selected_playlist_indices.insert(idx);
                                    }
                                }
                            }
                        }
                        KeyCode::Char('x') => {
                            self.stop_playback();
                        }
                        KeyCode::Char('p') => {
                            self.toggle_pause();
                        }
                        KeyCode::Left => {
                            self.seek(-5);
                        }
                        KeyCode::Right => {
                            self.seek(5);
                        }
                        KeyCode::Char('[') => {
                            self.seek(-30);
                        }
                        KeyCode::Char(']') => {
                            self.seek(30);
                        }
                        _ => {} // Ignore other keys in Normal state
                    },
                }
            }
            InputMode::Editing => {
                let control = key.modifiers.contains(KeyModifiers::CONTROL);
                match key.code {
                    KeyCode::Enter => {
                        self.perform_search();
                    }
                    KeyCode::Char(c) => {
                        if control {
                            match c {
                                'u' => {
                                    self.search_query.drain(..self.cursor_position);
                                    self.cursor_position = 0;
                                }
                                'k' => {
                                    self.search_query.truncate(self.cursor_position);
                                }
                                'w' | 'h' => {
                                    self.delete_word_backwards();
                                }
                                'a' => {
                                    self.cursor_position = 0;
                                }
                                'e' => {
                                    self.cursor_position = self.search_query.len();
                                }
                                _ => {} // Ignore other control chars
                            }
                        } else {
                            self.search_query.insert(self.cursor_position, c);
                            self.cursor_position += 1;
                        }
                    }
                    KeyCode::Backspace => {
                        if control {
                            self.delete_word_backwards();
                        } else if self.cursor_position > 0 {
                            self.search_query.remove(self.cursor_position - 1);
                            self.cursor_position -= 1;
                        }
                    }
                    KeyCode::Delete => {
                        if self.cursor_position < self.search_query.len() {
                            self.search_query.remove(self.cursor_position);
                        }
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.search_query.len() {
                            self.cursor_position += 1;
                        }
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                    }
                    KeyCode::End => {
                        self.cursor_position = self.search_query.len();
                    }
                    KeyCode::Esc | KeyCode::Tab => {
                        self.input_mode = InputMode::Normal;
                    }
                    _ => {} // Ignore other keys in Editing mode
                }
            }
            InputMode::Loading => {
                if code == KeyCode::Esc || code == KeyCode::Char('x') {
                    self.terminal_loading = false;
                    self.terminal_loading_error = None;
                    self.input_mode = InputMode::Normal;
                }
            }
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.search_results.is_empty() {
            self.selected_result_index = None;
            return;
        }

        let len = if !self.search_results.is_empty() && !self.is_url_mode {
            self.search_results.len() + 1 // +1 for "Load More"
        } else {
            self.search_results.len()
        };
        let current = self.selected_result_index.unwrap_or(0);

        let new_index = if delta > 0 {
            (current + (delta as usize)).min(len - 1)
        } else {
            current.saturating_sub(delta.abs() as usize)
        };

        self.selected_result_index = Some(new_index);

        if new_index < self.search_results.len() {
            self.request_image_for_selection();
        }
    }

    fn request_image_for_selection(&self) {
        if let Some(idx) = self.selected_result_index {
            if let Some(video) = self.search_results.get(idx) {
                if !self.image_cache.contains(&video.id) {
                    if let Some(url) = &video.thumbnail_url {
                        let _ = self.image_tx.send((video.id.clone(), url.clone()));
                    }
                }
            }
        }
    }

    pub fn perform_search(&mut self) {
        if self.search_query.trim().is_empty() {
            return;
        }

        self.input_mode = InputMode::Normal;
        self.search_results.clear();
        self.selected_result_index = None;
        self.playlist_stack.clear();
        self.selected_playlist_indices.clear();
        self.search_progress = Some(0.0);
        self.is_searching = true;
        self.current_search_id += 1;
        self.status_message = Some(format!("Searching for '{}'...", self.search_query));

        let is_url =
            self.search_query.starts_with("http://") || self.search_query.starts_with("https://");
        self.is_url_mode = is_url;

        let mut is_direct_playlist_url = false;
        if is_url {
            is_direct_playlist_url = self.search_query.contains("list=")
                || self.search_query.contains("/playlist/");
            // Also include the broader check for playlist identifiers
            if !is_direct_playlist_url &&
               (self.search_query.contains("PL") || self.search_query.contains("UU") ||
                self.search_query.contains("FL") || self.search_query.contains("RD") ||
                self.search_query.contains("OL")) {
                is_direct_playlist_url = true;
            }
        }

        self.search_offset = 1; // Always reset offset for new search

        if is_url && is_direct_playlist_url {
            // For direct playlist URLs, fetch multiple items
            let _ = self
                .search_tx
                .send((self.search_query.clone(), 1, 100, self.current_search_id));
        } else if is_url {
            // For other single item URLs
            let _ = self
                .search_tx
                .send((self.search_query.clone(), 1, 1, self.current_search_id));
        } else {
            // For regular text searches
            let _ = self
                .search_tx
                .send((self.search_query.clone(), 1, 20, self.current_search_id));
        }
    }

    pub fn load_more(&mut self) {
        if self.is_searching || self.search_query.trim().is_empty() {
            return;
        }

        self.is_searching = true;
        self.search_offset += 20;
        self.search_progress = Some(0.0);
        self.status_message = Some("Loading more...".to_string());

        let _ = self.search_tx.send((
            self.search_query.clone(),
            self.search_offset,
            self.search_offset + 19,
            self.current_search_id,
        ));
    }

    pub fn handle_paste(&mut self, text: String) {
        if self.input_mode == InputMode::Editing {
            self.search_query.insert_str(self.cursor_position, &text);
            self.cursor_position += text.len();
        } else {
            // Auto-switch to editing and paste
            self.input_mode = InputMode::Editing;
            self.search_query.insert_str(self.cursor_position, &text);
            self.cursor_position += text.len();
        }
    }

    pub fn stop_playback(&mut self) {
        if let Some(mut child) = self.playback_process.take() {
            let _ = child.start_kill();
        }
        self.playback_cmd_tx = None;
        self.playback_title = None;
        self.playback_time = 0.0;
        self.playback_total = 0.0;
        self.playback_duration_str = None;
        self.is_paused = false;
        self.is_finishing = false;
        self.terminal_loading = false;
        self.terminal_loading_error = None;
        self.terminal_ready_url = None;
        self.status_message = Some("Stopped.".to_string());
    }

    pub fn toggle_pause(&mut self) {
        if self.playback_cmd_tx.is_some() {
            self.is_paused = !self.is_paused;
            // mpv IPC expects JSON: { "command": ["cycle", "pause"] }
            self.send_command("{\"command\": [\"cycle\", \"pause\"]}\n");
            self.status_message = Some(if self.is_paused {
                "Paused".to_string()
            } else {
                "Resumed".to_string()
            });
        }
    }

    pub fn seek(&mut self, seconds: i32) {
        if self.playback_cmd_tx.is_some() {
            // mpv IPC: { "command": ["osd-msg-bar", "seek", seconds, "relative"] }
            let cmd = format!(
                "{{\"command\": [\"osd-msg-bar\", \"seek\", {}, \"relative\"]}}\n",
                seconds
            );
            self.send_command(&cmd);
            self.status_message = Some(format!("Seeked {}s", seconds));
        }
    }

    fn send_command(&self, cmd: &str) {
        if let Some(tx) = &self.playback_cmd_tx {
            let _ = tx.send(cmd.to_string());
        }
    }

    fn delete_word_backwards(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let mut chars = self.search_query[..self.cursor_position]
            .char_indices()
            .rev()
            .peekable();

        // Skip initial whitespace
        while let Some(&(_, c)) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        // Skip the word
        while let Some(&(_, c)) = chars.peek() {
            if !c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        let new_pos = chars.peek().map(|(i, _)| i + 1).unwrap_or(0);
        self.search_query.drain(new_pos..self.cursor_position);
        self.cursor_position = new_pos;
    }
}