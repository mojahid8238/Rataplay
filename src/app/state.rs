use crate::model::download::DownloadTask;
use crate::model::Video;
use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
    Loading, 
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
    PlayLocalTerminal, 
    PlayLocalAudio,    
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
    Settings,
}

#[derive(Debug)]
pub enum DownloadControl {
    Pause(String), 
    Resume(String), 
    Cancel(String), 
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
