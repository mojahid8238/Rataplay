use crate::model::Video;
use crate::sys::{image as sys_image, yt};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use image::DynamicImage;
use std::collections::HashMap;
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

#[derive(Debug, PartialEq, Eq)]
pub enum AppState {
    Search,
    Results,
    ActionMenu,
    FormatSelection,
}

pub struct App {
    pub running: bool,
    pub input_mode: InputMode,
    pub state: AppState,

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

    // Messages/Status
    pub status_message: Option<String>,

    // Actions
    pub actions: Vec<Action>,
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
    pub download_tx: UnboundedSender<(String, String)>, // URL, FormatID
    pub download_rx: UnboundedReceiver<crate::sys::download::DownloadProgress>,
    pub download_progress: Option<f32>,
    pub download_status: Option<String>,

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
                        if let Err(e) = yt::search_videos(&query, start, end, item_tx.clone()).await
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

        let (download_tx, mut download_cmd_rx) = mpsc::unbounded_channel::<(String, String)>();
        let (download_prog_tx, download_rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some((url, format_id)) = download_cmd_rx.recv().await {
                // We create a new channel for each download OR just share one?
                // Sharing one is easier for single download at a time.
                // But we need to pass a clone of download_prog_tx
                let tx = download_prog_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        crate::sys::download::download_video(url, format_id, tx.clone()).await
                    {
                        let _ =
                            tx.send(crate::sys::download::DownloadProgress::Error(e.to_string()));
                    }
                });
            }
        });

        let (_, playback_res_rx) = mpsc::unbounded_channel();
        let (terminal_ready_tx, terminal_ready_rx) =
            mpsc::unbounded_channel::<Result<String, String>>();

        Self {
            running: true,
            input_mode: InputMode::Editing,
            state: AppState::Search,
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
            status_message: None,
            actions: App::get_available_actions(),
            pending_action: None,
            image_tx,
            image_rx,
            image_cache: HashMap::new(),
            format_tx,
            format_rx,
            formats: Vec::new(),
            selected_format_index: None,
            download_tx,
            download_rx,
            download_progress: None,
            download_status: None,
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
        }
    }
    fn get_available_actions() -> Vec<Action> {
        vec![
            Action::new(
                KeyCode::Char('w'),
                "Watch (External)",
                AppAction::WatchExternal,
            ),
            Action::new(
                KeyCode::Char('t'),
                "Watch (In Terminal)",
                AppAction::WatchInTerminal,
            ),
            Action::new(
                KeyCode::Char('a'),
                "Listen (Audio Only)",
                AppAction::ListenAudio,
            ),
            Action::new(KeyCode::Char('d'), "Download", AppAction::Download),
        ]
    }

    pub fn on_tick(&mut self) {
        // check for search results
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                Ok((item, id)) => {
                    if id != self.current_search_id {
                        continue;
                    }
                    match item {
                        yt::SearchResult::Video(video) => {
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
                                self.status_message = Some("Results updated.".to_string());
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

        // check for images AND trigger new downloads if necessary
        while let Ok((id, img)) = self.image_rx.try_recv() {
            self.image_cache.insert(id, img);
        }

        // Trigger download for selected item if needed
        if let Some(idx) = self.selected_result_index {
            if let Some(video) = self.search_results.get(idx) {
                if !self.image_cache.contains_key(&video.id) {
                    if !self.image_cache.contains_key(&video.id) {
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

        // Check for download progress
        while let Ok(progress) = self.download_rx.try_recv() {
            match progress {
                crate::sys::download::DownloadProgress::Started => {
                    self.download_progress = Some(0.0);
                    self.download_status = Some("Starting download...".to_string());
                }
                crate::sys::download::DownloadProgress::Progress(pct, status) => {
                    self.download_progress = Some(pct / 100.0);
                    self.download_status = Some(status);
                }
                crate::sys::download::DownloadProgress::Finished => {
                    self.download_progress = None;
                    self.download_status = Some("Download Complete!".to_string());
                }
                crate::sys::download::DownloadProgress::Error(e) => {
                    self.download_progress = None;
                    self.download_status = Some(format!("Download Error: {}", e));
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
                            // Download selected format
                            if let Some(idx) = self.selected_format_index {
                                if let Some(fmt) = self.formats.get(idx) {
                                    if let Some(res_idx) = self.selected_result_index {
                                        if let Some(video) = self.search_results.get(res_idx) {
                                            // Start background download
                                            let _ = self
                                                .download_tx
                                                .send((video.url.clone(), fmt.format_id.clone()));
                                            self.state = AppState::Results;
                                            self.status_message =
                                                Some("Download started...".to_string());
                                            return;
                                        }
                                    }
                                }
                            }
                            self.state = AppState::Results; // Return to list after download start
                            self.status_message =
                                Some("Download started in background...".to_string());
                        }
                        _ => {}
                    },
                    AppState::ActionMenu => {
                        if code == KeyCode::Esc || code == KeyCode::Char('q') {
                            self.state = AppState::Results;
                            return;
                        }

                        if let Some(action) = self.actions.iter().find(|a| a.key == code) {
                            if let Some(idx) = self.selected_result_index {
                                if let Some(video) = self.search_results.get(idx) {
                                    let url = video.url.clone();
                                    let title = video.title.clone();
                                    match action.action {
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
                                            self.state = AppState::Results;
                                        }
                                        _ => {
                                            self.pending_action = Some((action.action, url, title));
                                            self.state = AppState::Results;
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
                                    self.state = AppState::ActionMenu;
                                } else {
                                    self.load_more();
                                }
                            }
                        }
                        KeyCode::Char('x') => {
                            self.stop_playback();
                        }
                        KeyCode::Char('p') | KeyCode::Char(' ') => {
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
                        _ => {}
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
                                _ => {}
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
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                    }
                    _ => {}
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

        let len = self.search_results.len() + 1; // +1 for "Load More"
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
                if !self.image_cache.contains_key(&video.id) {
                    if let Some(url) = &video.thumbnail_url {
                        let _ = self.image_tx.send((video.id.clone(), url.clone()));
                    }
                }
            }
        }
    }

    fn perform_search(&mut self) {
        if self.search_query.trim().is_empty() {
            return;
        }

        self.input_mode = InputMode::Normal;
        self.search_results.clear();
        self.selected_result_index = None;
        self.search_progress = Some(0.0);
        self.search_offset = 1;
        self.is_searching = true;
        self.current_search_id += 1;
        self.status_message = Some(format!("Searching for '{}'...", self.search_query));

        // Send query to background task
        let _ = self
            .search_tx
            .send((self.search_query.clone(), 1, 20, self.current_search_id));
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
