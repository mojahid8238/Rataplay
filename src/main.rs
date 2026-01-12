mod sys;
mod app;
mod tui;
mod model;

use std::process::exit;
use std::{io, time::{Duration, Instant}};
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Checking dependencies...");
    
    match sys::deps::check_dependencies() {
        Ok(status) => {
            println!("yt-dlp version: {}", status.yt_dlp_version);
            if !status.yt_dlp_up_to_date {
                 println!("WARNING: Your yt-dlp version is older than 14 days. Search might fail.");
                 println!("Recommendation: Run 'yt-dlp -U' to update.");
                 // We don't exit here, just warn, as per requirements ("suggest an update")
            }
            
            if !status.mpv_installed {
                eprintln!("CRITICAL: mpv is not installed or not in PATH.");
                eprintln!("Vivid requires mpv for playback.");
                exit(1);
            }
            
            println!("Environment check passed.");
        }
        Err(e) => {
            eprintln!("Dependency check failed: {}", e);
            exit(1);
        }
    }
    
    println!("Environment check passed. Starting Vivid...");
    
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize Image Picker
    // Try to detect, otherwise fallback.
    let mut picker = Picker::from_termios().unwrap_or_else(|_| {
        // Fallback to a font size of 8x16 roughly?
        // Or create without guessing
         Picker::new((8, 16))
    });
    // Guessing protocol might need manual intervention for Sixel, but from_termios tries hard.

    // Create App
    let mut app = App::new();

    // Main Loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| tui::ui(f, &mut app, &mut picker))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key_event(key);
            }
        }
        
        // Handle pending actions (Playback)
        if let Some(action) = app.pending_action.take() {
             // Suspend TUI
             execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            disable_raw_mode()?;
            terminal.show_cursor()?;

            // Run Action
            match action {
                app::AppAction::PlayVideo { url, in_terminal } => {
                    let full_url = if url.starts_with("http") { url } else { format!("https://youtu.be/{}", url) };
                    println!("Playing: {}", full_url);
                    if let Err(e) = sys::process::play_video(&full_url, in_terminal) {
                        eprintln!("Error playing video: {}", e);
                        std::thread::sleep(Duration::from_secs(3));
                    }
                }
                app::AppAction::PlayAudio { url } => {
                     let full_url = if url.starts_with("http") { url } else { format!("https://youtu.be/{}", url) };
                     println!("Playing Audio: {}", full_url);
                     if let Err(e) = sys::process::play_audio(&full_url) {
                         eprintln!("Error playing audio: {}", e);
                         std::thread::sleep(Duration::from_secs(3));
                     }
                }
                app::AppAction::Download { url, format_id } => {
                     let full_url = if url.starts_with("http") { url } else { format!("https://youtu.be/{}", url) };
                     println!("Downloading format {} from: {}", format_id, full_url);
                     if let Err(e) = sys::process::download_video(&full_url, &format_id) {
                         eprintln!("Error downloading: {}", e);
                         std::thread::sleep(Duration::from_secs(3));
                     }
                     println!("Download complete. Press Enter to continue.");
                     let mut _input = String::new();
                     let _ = std::io::stdin().read_line(&mut _input);
                }
            }
            
            // Resume TUI
            enable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                EnterAlternateScreen,
                EnableMouseCapture
            )?;
            terminal.hide_cursor()?;
            terminal.clear()?;
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        if !app.running {
            break;
        }
    }

    // Restore Terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
