mod app;
mod cli;
mod model;
mod sys;
mod tui;


use anyhow::Result;
use app::{App, AppAction, handle_key_event, handle_mouse_event, handle_paste, perform_search, stop_playback, on_tick};
use clap::Parser;
use cli::Cli;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::model::settings::Settings;
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;
use std::process::exit;
use std::{
    io,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

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

    // Load configuration early
    let config = crate::sys::config::Config::load();
    let settings = Settings::from_config(config.clone());

    // Initialize logging if enabled
    if config.logging.enabled {
        if let Ok(log_path) = config.get_log_path() {
            if let Err(e) = crate::sys::logging::init_logger(log_path) {
                eprintln!("Failed to initialize logger: {}", e);
            } else {
                log::info!("Rataplay starting up...");
                log::info!("Log path: {:?}", config.get_log_path().unwrap_or_default());
            }
        }
    }

    log::info!("Checking dependencies...");
    match sys::deps::check_dependencies(&settings) {
        Ok(status) => {
            // Run the update command to fetch live version info
            let update_check = std::process::Command::new(&settings.ytdlp_path).arg("-U").output();

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
    let mut app = App::new(config, settings.clone());

    // Handle startup query if provided
    if let Some(query) = args.query {
        app.search_query = query;
        perform_search(&mut app);
    }

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
                    Event::Key(key) => handle_key_event(&mut app, key),
                    Event::Paste(text) => handle_paste(&mut app, text),
                    Event::Mouse(mouse) => handle_mouse_event(&mut app, mouse),
                    _ => {}
                }
            }

            // Handle pending actions (Playback)
            if let Some((action, url, title)) = app.pending_action.take() {
                // Kill previous playback if any
                stop_playback(&mut app);

                // Suspend TUI only if needed (not needed for terminal anymore as it's separate)

                let full_url = url.clone();

                match action {
                    AppAction::WatchExternal => {
                        match sys::process::play_video(&full_url.to_string(), false, None, &settings) {
                            Ok(child) => {
                                let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                app.playback_res_rx = res_rx;

                                let socket_path = sys::mpv_ipc::get_ipc_path();
                                if !cfg!(windows) {
                                    let _ = std::fs::remove_file(&socket_path);
                                }

                                tokio::spawn(sys::mpv_ipc::spawn_ipc_handler(socket_path, rx, res_tx));

                                app.playback_cmd_tx = Some(tx);
                                app.playback_process = Some(child);
                                app.playback_title = Some(title);
                                app.status_message = Some("Playing externally...".to_string());
                                if let Some(mc) = &mut app.media_controller {
                                    let _ = mc.set_metadata(app.playback_title.as_deref().unwrap_or("Unknown"), None, None);
                                    let _ = mc.set_playback_status(true);
                                }
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Error playing video: {}", e));
                            }
                        }
                    }
                    AppAction::ListenAudio => {
                        match sys::process::play_audio(&full_url.to_string(), &settings) {
                            Ok(child) => {
                                let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                                app.playback_res_rx = res_rx;

                                let socket_path = sys::mpv_ipc::get_ipc_path();
                                if !cfg!(windows) {
                                    let _ = std::fs::remove_file(&socket_path);
                                }

                                tokio::spawn(sys::mpv_ipc::spawn_ipc_handler(socket_path, rx, res_tx));

                                app.playback_cmd_tx = Some(tx);
                                app.playback_process = Some(child);
                                app.playback_title = Some(title);
                                app.status_message = Some("Playing audio...".to_string());
                                if let Some(mc) = &mut app.media_controller {
                                    let _ = mc.set_metadata(app.playback_title.as_deref().unwrap_or("Unknown"), None, None);
                                    let _ = mc.set_playback_status(true);
                                }
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

                if let Ok(mut child) = sys::process::play_video(final_url, true, ua, &settings) {
                    // Update media controller
                    if let Some(mc) = &mut app.media_controller {
                        let _ = mc.set_metadata("Terminal Playback", None, None);
                        let _ = mc.set_playback_status(true);
                    }

                    // Set up IPC for terminal playback
                    let socket_path = sys::mpv_ipc::get_ipc_path();
                    if !cfg!(windows) {
                        let _ = std::fs::remove_file(&socket_path);
                    }
                    
                    // We need a separate task to write to the socket because the main loop is blocked here
                    // waiting for the child process.
                    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                    
                    // Spawn IPC writer task
                    let writer_handle = tokio::spawn(sys::mpv_ipc::spawn_ipc_writer(socket_path.clone(), cmd_rx));

                    // Event loop for terminal playback
                    loop {
                        tokio::select! {
                            // Check if child exited
                            status = child.wait() => {
                                let _ = status; 
                                break;
                            }
                            // Handle media events
                            Some(event) = app.media_rx.recv() => {
                                use sys::media::MediaEvent;
                                match event {
                                    MediaEvent::Play => {
                                        let _ = cmd_tx.send("{\"command\": [\"set_property\", \"pause\", false]}\n".to_string());
                                        if let Some(mc) = &mut app.media_controller {
                                            let _ = mc.set_playback_status(true);
                                        }
                                    }
                                    MediaEvent::Pause => {
                                        let _ = cmd_tx.send("{\"command\": [\"set_property\", \"pause\", true]}\n".to_string());
                                        if let Some(mc) = &mut app.media_controller {
                                            let _ = mc.set_playback_status(false);
                                        }
                                    }
                                    MediaEvent::Toggle => {
                                        let _ = cmd_tx.send("{\"command\": [\"cycle\", \"pause\"]}\n".to_string());
                                    }
                                    MediaEvent::Next => {
                                        let _ = cmd_tx.send("{\"command\": [\"seek\", 10, \"relative\"]}\n".to_string());
                                    }
                                    MediaEvent::Previous => {
                                        let _ = cmd_tx.send("{\"command\": [\"seek\", -10, \"relative\"]}\n".to_string());
                                    }
                                    MediaEvent::Stop => {
                                        let _ = child.start_kill();
                                    }
                                }
                            }
                        }
                    }
                    
                    // Cleanup
                    writer_handle.abort();
                    if !cfg!(windows) {
                        let _ = std::fs::remove_file(&socket_path);
                    }

                    if let Some(mc) = &mut app.media_controller {
                        let _ = mc.set_playback_status(false);
                    }
                }

                // Resume TUI
                enable_raw_mode()?;
                execute!(terminal.backend_mut(), EnableMouseCapture)?;
                terminal.hide_cursor()?;
                terminal.clear()?;
            }

            if last_tick.elapsed() >= tick_rate {
                on_tick(&mut app);
                last_tick = Instant::now();
            }

            if !app.running {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    }.await;

    // Restore Terminal
    stop_playback(&mut app);
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
