use crossterm::event::{KeyCode, KeyEvent};
use crate::model::Video;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use crate::sys::{yt, image as sys_image};
use image::DynamicImage;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Loading, // Added Loading state
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
    pub search_tx: UnboundedSender<String>,
    pub result_rx: UnboundedReceiver<Result<Vec<Video>, String>>,

    // Messages/Status
    pub status_message: Option<String>,
    
    // Actions
    pub pending_action: Option<AppAction>,
    
    // Images
    pub image_tx: UnboundedSender<(String, String)>, // (ID, URL)
    pub image_rx: UnboundedReceiver<(String, DynamicImage)>,
    pub image_cache: std::collections::HashMap<String, DynamicImage>,
    
    // Download / Formats
    pub format_tx: UnboundedSender<String>, // URL
    pub format_rx: UnboundedReceiver<Result<Vec<crate::model::VideoFormat>, String>>,
    pub formats: Vec<crate::model::VideoFormat>,
    pub selected_format_index: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppAction {
    PlayVideo { url: String, in_terminal: bool },
    PlayAudio { url: String },
    Download { url: String, format_id: String },
}

impl App {
    pub fn new() -> Self {
        let (search_tx, mut search_rx) = mpsc::unbounded_channel::<String>();
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        // Spawn a background task to handle search requests
        tokio::spawn(async move {
            while let Some(query) = search_rx.recv().await {
                 match yt::search_videos(&query).await {
                     Ok(videos) => {
                         let _ = result_tx.send(Ok(videos));
                     }
                     Err(e) => {
                         let _ = result_tx.send(Err(e.to_string()));
                     }
                 }
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

        Self {
            running: true,
            input_mode: InputMode::Normal,
            state: AppState::Search,
            search_query: String::new(),
            cursor_position: 0,
            search_results: Vec::new(),
            selected_result_index: None,
            search_tx,
            result_rx,
            status_message: None,
            pending_action: None,
            image_tx,
            image_rx,
            image_cache: HashMap::new(),
            format_tx,
            format_rx,
            formats: Vec::new(),
            selected_format_index: None,
        }
    }

    pub fn on_tick(&mut self) {
        // check for search results
        if let Ok(result) = self.result_rx.try_recv() {
             match result {
                 Ok(videos) => {
                     self.search_results = videos;
                     self.input_mode = InputMode::Normal;
                     self.state = AppState::Results;
                     if !self.search_results.is_empty() {
                         self.selected_result_index = Some(0);
                     }
                     self.status_message = Some("Search completed.".to_string());
                 }
                 Err(e) => {
                     self.input_mode = InputMode::Normal;
                     self.status_message = Some(format!("Error: {}", e));
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
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if self.input_mode == InputMode::Loading {
            return;
        }

        match self.input_mode {
            InputMode::Normal => {
                match self.state {
                    AppState::FormatSelection => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.state = AppState::ActionMenu;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if let Some(idx) = self.selected_format_index {
                                if idx > 0 { self.selected_format_index = Some(idx - 1); }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                             if let Some(idx) = self.selected_format_index {
                                if idx < self.formats.len().saturating_sub(1) { self.selected_format_index = Some(idx + 1); }
                             }
                        }
                        KeyCode::Enter => {
                            // Download selected format
                            if let Some(idx) = self.selected_format_index {
                                if let Some(fmt) = self.formats.get(idx) {
                                    if let Some(res_idx) = self.selected_result_index {
                                        if let Some(video) = self.search_results.get(res_idx) {
                                            self.pending_action = Some(AppAction::Download {
                                                url: video.id.clone(),
                                                format_id: fmt.format_id.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                            self.state = AppState::Results; // Return to list after download start
                            self.status_message = Some("Download started in background...".to_string());
                        }
                        _ => {}
                    },
                    AppState::ActionMenu => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.state = AppState::Results;
                        }
                        KeyCode::Char('d') => {
                           // Trigger Format Fetch
                           if let Some(idx) = self.selected_result_index {
                               if let Some(video) = self.search_results.get(idx) {
                                   let url = if video.id.starts_with("http") { video.id.clone() } else { format!("https://youtu.be/{}", video.id) };
                                   let _ = self.format_tx.send(url);
                                   self.input_mode = InputMode::Loading; // Wait for formats
                                   self.status_message = Some("Fetching formats...".to_string());
                               }
                           }
                        }
                        KeyCode::Char('t') => {
                            // Play in Terminal
                            if let Some(idx) = self.selected_result_index {
                                if let Some(video) = self.search_results.get(idx) {
                                    self.pending_action = Some(AppAction::PlayVideo { 
                                        url: video.id.clone(), // yt-dlp needs just ID or URL. ID works if we prefix or if yt-dlp handles it. safer to pass full URL or just ID? yt-dlp works with ID usually, mpv needs full url or ytdl hook. 
                                        // Wait, we are passing this to mpv directly.
                                        // mpv with ytdl hook can take "https://youtube.com/watch?v=ID" or just "ID" sometimes.
                                        // Let's construct a URL to be safe: "https://youtu.be/<id>"
                                        in_terminal: true 
                                    });
                                }
                            }
                            self.state = AppState::Results;
                        }
                        KeyCode::Char('w') | KeyCode::Enter => {
                             // Watch External (Default)
                             if let Some(idx) = self.selected_result_index {
                                if let Some(video) = self.search_results.get(idx) {
                                    self.pending_action = Some(AppAction::PlayVideo { 
                                        url: video.id.clone(),
                                        in_terminal: false
                                    });
                                }
                            }
                            self.state = AppState::Results;
                        }
                        KeyCode::Char('a') => {
                            // Audio
                            if let Some(idx) = self.selected_result_index {
                                if let Some(video) = self.search_results.get(idx) {
                                    self.pending_action = Some(AppAction::PlayAudio { 
                                        url: video.id.clone(),
                                    });
                                }
                            }
                            self.state = AppState::Results;
                        }
                        _ => {}
                    },
                    _ => match key.code {
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
                            if !self.search_results.is_empty() {
                                self.state = AppState::ActionMenu;
                            }
                        }
                        _ => {}
                    }
                }
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    self.perform_search();
                }
                KeyCode::Char(c) => {
                    self.search_query.insert(self.cursor_position, c);
                    self.cursor_position += 1;
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        self.search_query.remove(self.cursor_position - 1);
                        self.cursor_position -= 1;
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
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                _ => {}
            },
            InputMode::Loading => {} // Handled by guard at top of function
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.search_results.is_empty() {
            return;
        }

        let len = self.search_results.len();
        let current = self.selected_result_index.unwrap_or(0);
        
        let new_index = if delta > 0 {
            (current + (delta as usize)).min(len - 1)
        } else {
            current.saturating_sub(delta.abs() as usize)
        };

        self.selected_result_index = Some(new_index);
        self.request_image_for_selection();
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
        
        self.input_mode = InputMode::Loading;
        self.status_message = Some(format!("Searching for '{}'...", self.search_query));
        
        // Send query to background task
        let _ = self.search_tx.send(self.search_query.clone());
    }
}
