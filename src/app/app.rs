use crate::model::Video;
use crate::model::local::LocalFile;
use crate::sys::media::{MediaController, MediaEvent};
use crate::sys::{image as sys_image, local, yt};
use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui::widgets::{ListState, TableState};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::tui::components::logo::AnimationMode;
use crate::tui::components::theme::Theme;

use super::{AppAction, AppState, DownloadControl, DownloadManager, InputMode};
use crate::model::settings::Settings;

pub struct App {
    pub running: bool,
    pub input_mode: InputMode,
    pub state: AppState,
    pub previous_app_state: AppState,
    // Search
    pub search_query: String,
    pub cursor_position: usize,
    pub settings_input: String,
    pub settings_cursor_position: usize,
    pub search_limit: u32,
    pub playlist_limit: u32,
    // UI Layout Areas for Mouse Interaction
    pub search_bar_area: Rect,
    pub main_content_area: Rect,
    pub downloads_area: Option<Rect>,
    pub playback_bar_area: Option<Rect>,
    pub action_menu_area: Option<Rect>,
    pub format_selection_area: Option<Rect>,
    pub settings_area: Option<Rect>,
    pub settings_editing_item: Option<crate::tui::components::settings::SettingItem>,

    // UI States (persisted for scroll offset tracking)
    pub main_list_state: ListState,
    pub downloads_active_state: TableState,
    pub downloads_local_state: TableState,
    pub action_menu_state: ListState,
    pub format_selection_state: TableState,
    pub settings_state: ListState,

    // Mouse Tracking
    pub last_click_time: Option<Instant>,
    pub last_click_pos: Option<(u16, u16)>,

    // Animation State
    pub pet_frame: usize,

    // Visuals
    pub theme: Theme,
    pub theme_index: usize,
    pub download_directory: String,
    pub animation_mode: AnimationMode,
    pub show_live: bool,
    pub show_playlists: bool,
    pub settings: Settings,

    // Results
    pub search_results: Vec<Video>,
    pub selected_result_index: Option<usize>,
    // Async Communication
    pub search_tx: UnboundedSender<(String, u32, u32, usize, bool, bool)>, // query, start, end, search_id, show_live, show_playlists
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
    pub image_cache: std::collections::HashMap<String, DynamicImage>,
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

    // Clipboard
    pub clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn change_theme(&mut self) {
        self.theme_index =
            (self.theme_index + 1) % crate::tui::components::theme::AVAILABLE_THEMES.len();
        self.theme = crate::tui::components::theme::AVAILABLE_THEMES[self.theme_index];
        self.status_message = Some(format!("Theme: {}", self.theme.name));

        self.save_config();
    }

    pub fn toggle_animation(&mut self) {
        let all = AnimationMode::all();
        let current_idx = all
            .iter()
            .position(|m| m == &self.animation_mode)
            .unwrap_or(0);
        self.animation_mode = all[(current_idx + 1) % all.len()];
        self.status_message = Some(format!("Animation: {}", self.animation_mode.name()));

        self.save_config();
    }

    pub fn toggle_live(&mut self) {
        self.show_live = !self.show_live;
        self.status_message = Some(format!(
            "Show Live: {}",
            if self.show_live { "On" } else { "Off" }
        ));
        self.save_config();

        if self.state == AppState::Results && !self.is_url_mode {
            crate::app::actions::perform_search(self);
        }
    }

    pub fn toggle_playlists(&mut self) {
        self.show_playlists = !self.show_playlists;
        self.status_message = Some(format!(
            "Show Playlists: {}",
            if self.show_playlists { "On" } else { "Off" }
        ));
        self.save_config();

        if self.state == AppState::Results && !self.is_url_mode {
            crate::app::actions::perform_search(self);
        }
    }

    pub fn save_config(&self) {
        let config = crate::sys::config::Config {
            theme: self.theme.name.to_string(),
            search_limit: self.search_limit,
            playlist_limit: self.playlist_limit,
            download_directory: self.download_directory.clone(),
            animation: self.animation_mode,
            show_live: self.show_live,
            show_playlists: self.show_playlists,
            executables: crate::sys::config::Executables {
                enabled: self.settings.use_custom_paths,
                mpv: if self.settings.mpv_path == "mpv" { None } else { Some(std::path::PathBuf::from(&self.settings.mpv_path)) },
                ytdlp: if self.settings.ytdlp_path == "yt-dlp" { None } else { Some(std::path::PathBuf::from(&self.settings.ytdlp_path)) },
                ffmpeg: if self.settings.ffmpeg_path == "ffmpeg" { None } else { Some(std::path::PathBuf::from(&self.settings.ffmpeg_path)) },
                deno: if self.settings.deno_path == "deno" { None } else { Some(std::path::PathBuf::from(&self.settings.deno_path)) },
            },
            cookies: crate::sys::config::Cookies {
                enabled: !matches!(self.settings.cookie_mode, crate::model::settings::CookieMode::Off),
                source: match &self.settings.cookie_mode {
                    crate::model::settings::CookieMode::Off => crate::sys::config::CookieSource::Disabled,
                    crate::model::settings::CookieMode::File(p) => crate::sys::config::CookieSource::Netscape(p.clone()),
                    crate::model::settings::CookieMode::Browser(b) => crate::sys::config::CookieSource::Browser(b.clone()),
                }
            },
            logging: crate::sys::config::Logging {
                enabled: self.settings.enable_logging,
                path: self.settings.log_path.clone(),
            },
        };
        let _ = config.save();
    }

    pub fn new(config: crate::sys::config::Config, settings: Settings) -> Self {
        // Only save if config file doesn't exist to generate default template
        // This prevents overwriting user's custom comments/formatting on every startup
        let config_path = crate::sys::config::Config::get_config_path();
        if !config_path.exists() {
             let _ = config.save();
        }

        let (theme_index, theme) = crate::tui::components::theme::AVAILABLE_THEMES
            .iter()
            .enumerate()
            .find(|(_, t)| t.name == config.theme)
            .map(|(i, t)| (i, *t))
            .unwrap_or((0, crate::tui::components::theme::AVAILABLE_THEMES[0]));

        let (search_tx, mut search_rx) =
            mpsc::unbounded_channel::<(String, u32, u32, usize, bool, bool)>();
        let (result_tx, result_rx) =
            mpsc::unbounded_channel::<Result<(yt::SearchResult, usize), String>>();

        let settings_clone = settings.clone();
        tokio::spawn(async move {
            while let Some((query, start, end, id, show_live, show_playlists)) =
                search_rx.recv().await
            {
                let tx = result_tx.clone();
                let settings_inner = settings_clone.clone();
                tokio::spawn(async move {
                    let (item_tx, mut item_rx) = mpsc::unbounded_channel();

                    let search_handle = tokio::spawn(async move {
                        if let Err(e) = yt::search_videos_flat(
                            &query,
                            start,
                            end,
                            show_live,
                            show_playlists,
                            settings_inner,
                            item_tx.clone(),
                        )
                        .await
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
                let res_tx = image_res_tx.clone();
                tokio::spawn(async move {
                    if let Ok(img) = sys_image::download_image(&url, &id).await {
                        let _ = res_tx.send((id, img));
                    }
                });
            }
        });

        let (format_tx, mut format_req_rx) = mpsc::unbounded_channel::<String>();
        let (format_res_tx, format_rx) = mpsc::unbounded_channel();

        let settings_clone = settings.clone();
        tokio::spawn(async move {
            while let Some(url) = format_req_rx.recv().await {
                match yt::get_video_formats(&url, &settings_clone).await {
                    Ok(formats) => {
                        let _ = format_res_tx.send(Ok(formats));
                    }
                    Err(e) => {
                        let _ = format_res_tx.send(Err(e.to_string()));
                    }
                }
            }
        });

        let (new_download_tx, mut new_download_cmd_rx) =
            mpsc::unbounded_channel::<(Video, String)>();
        let (download_event_tx, download_event_rx) = mpsc::unbounded_channel();
        let (download_control_tx, mut download_control_rx) =
            mpsc::unbounded_channel::<DownloadControl>();

        use libc::{SIGCONT, SIGSTOP, SIGTERM, kill};
        use std::collections::HashMap;
        use tokio::io::{AsyncBufReadExt, BufReader};

        // Spawn a background task to handle download requests and control messages
        let resolved_download_dir = local::resolve_path(&config.download_directory)
            .to_string_lossy()
            .to_string();
        let settings_clone = settings.clone();
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
                            let mut child = match crate::sys::download::start_download(&video, &format_id, &resolved_download_dir, &settings_clone).await {
                                Ok(child) => child,
                                Err(e) => {
                                    log::error!("Failed to start download for video {}: {}", video_id, e);
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
                                log::debug!("Monitoring download for video: {}", video_id);

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
                                        Ok(Some(line)) = stderr_reader.next_line() => {
                                            log::warn!("yt-dlp stderr for {}: {}", video_id, line);
                                        }
                                        status = child.wait() => {
                                            match status {
                                                Ok(exit_status) => {
                                                    if exit_status.success() {
                                                        log::info!("Download finished successfully for video: {}", video_id);
                                                        let _ = monitor_event_tx.send(crate::model::download::DownloadEvent::Finished(video_id.clone()));
                                                    } else {
                                                        log::error!("Download failed for video {}: exit code {:?}", video_id, exit_status.code());
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

        let settings_clone = settings.clone();
        tokio::spawn(async move {
            while let Some(ids) = details_req_rx.recv().await {
                let res_tx = details_res_tx.clone();
                if let Err(e) = yt::resolve_video_details(ids, settings_clone.clone(), res_tx.clone()).await {
                    let _ = res_tx.send(Err(e.to_string()));
                }
            }
        });

        // Scan local files initially
        let download_path_buf = local::resolve_path(&config.download_directory);
        let download_path = download_path_buf.as_path();
        let local_files = local::scan_local_files(download_path);
        let mut download_manager = DownloadManager::new();

        // Scan for incomplete downloads to resume
        let incomplete = local::scan_incomplete_downloads(download_path);
        for (id, title, url, format_id) in incomplete {
            if !download_manager.tasks.contains_key(&id) {
                let mut video = Video::default();
                video.id = id.clone();
                video.title = title.clone();
                video.url = url;

                let mut task = crate::model::download::DownloadTask::new(video, format_id);
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
            settings_input: String::new(),
            settings_cursor_position: 0,
            search_limit: config.search_limit,
            playlist_limit: config.playlist_limit,
            search_bar_area: Rect::default(),
            main_content_area: Rect::default(),
            downloads_area: None,
            playback_bar_area: None,
            action_menu_area: None,
            format_selection_area: None,
            settings_area: None,
            settings_editing_item: None,

            main_list_state: ListState::default(),
            downloads_active_state: TableState::default(),
            downloads_local_state: TableState::default(),
            action_menu_state: ListState::default(),
            format_selection_state: TableState::default(),
            settings_state: ListState::default(),

            last_click_time: None,
            last_click_pos: None,

            pet_frame: 0,

            theme,
            theme_index,
            download_directory: config.download_directory,
            animation_mode: config.animation,
            show_live: config.show_live,
            show_playlists: config.show_playlists,
            settings,

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
            image_cache: std::collections::HashMap::new(),
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
            clipboard: arboard::Clipboard::new().ok(),
        }
    }
}
