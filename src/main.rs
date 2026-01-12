mod app;
mod model;
mod sys;
mod tui;

use anyhow::Result;
use app::{App, AppAction};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;
use std::process::exit;
use std::{
    io,
    time::{Duration, Instant},
};

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
        if let Some((action, url)) = app.pending_action.take() {
            let in_terminal = matches!(action, AppAction::WatchInTerminal);

            // Suspend TUI only if needed
            if in_terminal {
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                disable_raw_mode()?;
                terminal.show_cursor()?;
            }

            let full_url = if url.starts_with("http") {
                url.clone()
            } else {
                format!("https://youtu.be/{}", url)
            };

            match action {
                AppAction::WatchInTerminal => {
                    println!("Playing in terminal: {}", full_url);
                    if let Err(e) = sys::process::play_video(&full_url.to_string(), true) {
                        eprintln!("Error playing video: {}", e);
                        std::thread::sleep(Duration::from_secs(3));
                    }
                }
                AppAction::WatchExternal => {
                    println!("Playing externally: {}", full_url);
                    // Spawn in a separate thread to not block the UI
                    std::thread::spawn(move || {
                        let _ = sys::process::play_video(&full_url.to_string(), false);
                    });
                }
                AppAction::ListenAudio => {
                    println!("Playing audio: {}", full_url);
                    // Spawn in a separate thread
                    std::thread::spawn(move || {
                        let _ = sys::process::play_audio(&full_url.to_string());
                    });
                }
                AppAction::Download => {
                    // This is handled in the app logic by switching to FormatSelection
                    // It shouldn't get here.
                }
            }
            if in_terminal {
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
