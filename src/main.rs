mod app;
mod model;
mod sys;
mod tui;

use anyhow::Result;
use app::{App, AppAction};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::{Picker, ProtocolType};
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
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize Image Picker with specialized detection for Kitty/WezTerm
    let mut picker = Picker::from_termios().unwrap_or_else(|_| Picker::new((8, 16)));

    // Explicitly check for Kitty/WezTerm to enable advanced graphics protocol
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    if term == "xterm-kitty" || term_program == "WezTerm" {
        // Force Kitty protocol if detected to avoid pixelation
        picker.protocol_type = ProtocolType::Kitty;
    }

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
            match event::read()? {
                Event::Key(key) => app.handle_key_event(key),
                Event::Paste(text) => app.handle_paste(text),
                _ => {}
            }
        }

        // Handle pending actions (Playback)
        if let Some((action, url, title)) = app.pending_action.take() {
            // Kill previous playback if any
            app.stop_playback();

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

            let full_url = url.clone();

            match action {
                AppAction::WatchInTerminal => {
                    if let Ok(mut child) = sys::process::play_video(&full_url.to_string(), true) {
                        let _ = child.wait().await;
                    }
                }
                AppAction::WatchExternal => {
                    match sys::process::play_video(&full_url.to_string(), false) {
                        Ok(child) => {
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                            let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                            app.playback_res_rx = res_rx;

                            let socket_path = format!("/tmp/vivid-mpv-{}.sock", std::process::id());

                            tokio::spawn(async move {
                                use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                                // Wait a bit for mpv to create the socket
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                                if let Ok(stream) =
                                    tokio::net::UnixStream::connect(&socket_path).await
                                {
                                    let (reader, mut writer) = stream.into_split();
                                    let mut reader = BufReader::new(reader);

                                    // Spawning reader task
                                    let reader_handle = tokio::spawn(async move {
                                        let mut line = String::new();
                                        while let Ok(n) = reader.read_line(&mut line).await {
                                            if n == 0 {
                                                break;
                                            }
                                            let _ = res_tx.send(line.clone());
                                            line.clear();
                                        }
                                    });

                                    // Writer loop
                                    while let Some(cmd) = rx.recv().await {
                                        let _ = writer.write_all(cmd.as_bytes()).await;
                                        let _ = writer.flush().await;
                                    }
                                    let _ = reader_handle.abort();
                                }
                                let _ = tokio::fs::remove_file(&socket_path).await;
                            });

                            app.playback_cmd_tx = Some(tx);
                            app.playback_process = Some(child);
                            app.playback_title = Some(title);
                            app.status_message = Some("Playing externally...".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error playing video: {}", e));
                        }
                    }
                }
                AppAction::ListenAudio => {
                    match sys::process::play_audio(&full_url.to_string()) {
                        Ok(child) => {
                            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                            let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                            app.playback_res_rx = res_rx;

                            let socket_path = format!("/tmp/vivid-mpv-{}.sock", std::process::id());

                            tokio::spawn(async move {
                                use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
                                // Wait a bit for mpv to create the socket
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                                if let Ok(stream) =
                                    tokio::net::UnixStream::connect(&socket_path).await
                                {
                                    let (reader, mut writer) = stream.into_split();
                                    let mut reader = BufReader::new(reader);

                                    // Spawning reader task
                                    let reader_handle = tokio::spawn(async move {
                                        let mut line = String::new();
                                        while let Ok(n) = reader.read_line(&mut line).await {
                                            if n == 0 {
                                                break;
                                            }
                                            let _ = res_tx.send(line.clone());
                                            line.clear();
                                        }
                                    });

                                    // Writer loop
                                    while let Some(cmd) = rx.recv().await {
                                        let _ = writer.write_all(cmd.as_bytes()).await;
                                        let _ = writer.flush().await;
                                    }
                                    let _ = reader_handle.abort();
                                }
                                let _ = tokio::fs::remove_file(&socket_path).await;
                            });

                            app.playback_cmd_tx = Some(tx);
                            app.playback_process = Some(child);
                            app.playback_title = Some(title);
                            app.status_message = Some("Playing audio...".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error playing audio: {}", e));
                        }
                    }
                }
                AppAction::Download => {}
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
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    app.stop_playback();

    Ok(())
}
