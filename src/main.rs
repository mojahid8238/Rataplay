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
use ratatui_image::picker::Picker;
use std::process::exit;
use std::{
    io,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Set panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = std::io::stdout();
        let _ = disable_raw_mode();
        let _ = execute!(
            stdout,
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableBracketedPaste,
            crossterm::cursor::Show
        );
        original_hook(panic_info);
    }));

    println!("Checking dependencies...");

    match sys::deps::check_dependencies() {
        Ok(status) => {
            // Run the update command to fetch live version info
            let update_check = std::process::Command::new("yt-dlp").arg("-U").output();

            match update_check {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);

                    let mut current_version: Option<String> = None;
                    let mut latest_version: Option<String> = None;

                    for line in stdout.lines() {
                        if line.contains("Current version:") {
                            current_version = line
                                .split("Current version: ")
                                .nth(1)
                                .and_then(|s| s.split(" ").next())
                                .map(|s| s.to_string());
                        } else if line.contains("Latest version:") {
                            latest_version = line
                                .split("Latest version: ")
                                .nth(1)
                                .and_then(|s| s.split(" ").next())
                                .map(|s| s.to_string());
                        }
                    }

                    if let (Some(current), Some(latest)) = (current_version, latest_version) {
                        if current < latest {
                            println!(
                                "⚠️  UPDATE REQUIRED: You are behind the latest yt-dlp release."
                            );
                            println!("--------------------------------------------------");
                            println!("🚀 Latest version: {}", latest);
                            println!("--------------------------------------------------");
                            println!("CRITICAL: You must update to ensure search and playback features work.");
                            println!(
                                "yt-dlp extractors change daily; being behind may cause failures."
                            );
                        } else {
                            println!(
                                "✅ yt-dlp is up to date (Version: {})",
                                status.yt_dlp_version
                            );
                        }
                    } else if stdout.contains("is up to date") {
                        // Fallback for when `yt-dlp -U` output differs but still indicates up to date
                        println!(
                            "✅ yt-dlp is up to date (Version: {})",
                            status.yt_dlp_version
                        );
                    } else {
                        // Fallback for unexpected output or permission errors
                        println!("yt-dlp version: {}", status.yt_dlp_version);
                        println!(
                            "⚠️  Could not verify update status. Please run 'yt-dlp -U' manually."
                        );
                    }
                }
                Err(_) => {
                    // Fallback for when the command fails to execute (e.g., no internet or binary missing)
                    println!("yt-dlp version: {}", status.yt_dlp_version);
                    println!("❌ WARNING: Unable to check for updates. Please ensure you are on the latest version");
                    println!("   to prevent search and download features from breaking.");
                }
            }
            // Secondary dependency checks
            if !status.mpv_installed {
                eprintln!("CRITICAL: mpv is not installed or not in PATH.");
                eprintln!("❌ WARNING: Rataplay requires mpv for playback.");
                exit(1);
            }
        }
        Err(e) => {
            eprintln!("Dependency check failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("Environment check passed. Starting Rataplay...");

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
    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    // Explicitly check for Kitty/WezTerm to enable advanced graphics protocol
    let term = std::env::var("TERM").unwrap_or_default();
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    if term == "xterm-kitty" || term_program == "WezTerm" {
        // Force Kitty protocol if detected to avoid pixelation
        picker.set_protocol_type(ratatui_image::picker::ProtocolType::Kitty);
    }

    // Create App
    let mut app = App::new();

    // Main Loop
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    let run_result = async {
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

                // Suspend TUI only if needed (not needed for terminal anymore as it's separate)

                let full_url = url.clone();

                match action {
                    AppAction::WatchExternal => {
                        match sys::process::play_video(&full_url.to_string(), false, None) {
                            Ok(child) => {
                                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                app.playback_res_rx = res_rx;

                                let socket_path =
                                    format!("/tmp/rataplay-mpv-{}.sock", std::process::id());
                                let _ = std::fs::remove_file(&socket_path);

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

                                let socket_path =
                                    format!("/tmp/rataplay-mpv-{}.sock", std::process::id());
                                let _ = std::fs::remove_file(&socket_path);

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
                    _ => {}
                }
            }

            // Handle Terminal Playback readiness
            if let Some(url) = app.terminal_ready_url.take() {
                app.terminal_loading = false;
                app.terminal_loading_progress = 0.0;

                // Suspend TUI logic but stay in Alternate Screen
                execute!(terminal.backend_mut(), DisableMouseCapture)?;
                disable_raw_mode()?;
                terminal.show_cursor()?;

                // Play video (direct URL is faster)
                let (final_url, ua) = if url.starts_with("http") && url.contains('|') {
                    let parts: Vec<&str> = url.splitn(2, '|').collect();
                    (parts[0], Some(parts[1]))
                } else {
                    (url.as_str(), None)
                };

                if let Ok(mut child) = sys::process::play_video(final_url, true, ua) {
                    let _ = child.wait().await;
                }

                // Resume TUI
                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnableMouseCapture)?;
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
        Ok::<(), anyhow::Error>(())
    }.await;

    // Restore Terminal
    app.stop_playback();
    cleanup_terminal(&mut terminal)?;

    if let Err(err) = run_result {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste,
        crossterm::cursor::Show
    )?;
    Ok(())
}
