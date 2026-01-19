use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, PlatformConfig};
use tokio::sync::mpsc::UnboundedSender;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum MediaEvent {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
}

pub struct MediaController {
    controls: MediaControls,
}

impl MediaController {
    pub fn init(tx: UnboundedSender<MediaEvent>) -> Result<Self, souvlaki::Error> {
        #[cfg(target_os = "windows")]
        let hwnd = None; // Needs a real window handle on Windows, or use a dummy window
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        let config = PlatformConfig {
            dbus_name: "rataplay",
            display_name: "Rataplay",
            hwnd,
        };

        let mut controls = MediaControls::new(config)?;

        // Connect the event handler
        controls.attach(move |event: MediaControlEvent| {
            let app_event = match event {
                MediaControlEvent::Play => MediaEvent::Play,
                MediaControlEvent::Pause => MediaEvent::Pause,
                MediaControlEvent::Toggle => MediaEvent::Toggle,
                MediaControlEvent::Next => MediaEvent::Next,
                MediaControlEvent::Previous => MediaEvent::Previous,
                MediaControlEvent::Stop => MediaEvent::Stop,
                _ => return, // Ignore others for now
            };
            let _ = tx.send(app_event);
        })?;

        Ok(Self { controls })
    }

    pub fn set_playback_status(&mut self, playing: bool) -> Result<(), souvlaki::Error> {
        self.controls.set_playback(if playing {
            souvlaki::MediaPlayback::Playing { progress: None }
        } else {
            souvlaki::MediaPlayback::Paused { progress: None }
        })
    }

    pub fn set_metadata(
        &mut self,
        title: &str,
        artist: Option<&str>,
        duration: Option<Duration>,
    ) -> Result<(), souvlaki::Error> {
        let metadata = MediaMetadata {
            title: Some(title),
            artist,
            album: None,
            duration,
            cover_url: None,
        };
        self.controls.set_metadata(metadata)
    }
}
