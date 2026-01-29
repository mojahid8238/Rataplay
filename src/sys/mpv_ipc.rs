use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc::UnboundedSender;

pub fn get_ipc_path() -> String {
    if cfg!(windows) {
        format!(r"\\\\.\\pipe\\rataplay-mpv-{}", std::process::id())
    } else {
        format!("/tmp/rataplay-mpv-{}.sock", std::process::id())
    }
}

pub async fn spawn_ipc_handler(
    socket_path: String,
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
    res_tx: UnboundedSender<String>,
) -> Result<()> {
    // Wait a bit for mpv to create the socket
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    #[cfg(unix)]
    {
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(stream) => {
                log::info!("Connected to MPV IPC socket: {}", socket_path);
                let (reader, mut writer) = stream.into_split();
                let mut reader = BufReader::new(reader);

                // Spawning reader task
                let reader_handle = tokio::spawn(async move {
                    let mut line = String::new();
                    while let Ok(n) = reader.read_line(&mut line).await {
                        if n == 0 { break; }
                        let _ = res_tx.send(line.clone());
                        line.clear();
                    }
                });

                // Writer loop
                while let Some(cmd) = cmd_rx.recv().await {
                    let _ = writer.write_all(cmd.as_bytes()).await;
                    let _ = writer.flush().await;
                }
                let _ = reader_handle.abort();
            }
            Err(e) => {
                log::error!("Failed to connect to MPV IPC socket {}: {}", socket_path, e);
            }
        }
    }

    #[cfg(windows)]
    {
        use tokio::net::windows::named_pipe::ClientOptions;
        if let Ok(client) = ClientOptions::new().open(&socket_path) {
            let (reader, mut writer) = split(client);
            let mut reader = BufReader::new(reader);

            // Spawning reader task
            let reader_handle = tokio::spawn(async move {
                let mut line = String::new();
                while let Ok(n) = reader.read_line(&mut line).await {
                    if n == 0 { break; }
                    let _ = res_tx.send(line.clone());
                    line.clear();
                }
            });

            // Writer loop
            while let Some(cmd) = cmd_rx.recv().await {
                let _ = writer.write_all(cmd.as_bytes()).await;
                let _ = writer.flush().await;
            }
            let _ = reader_handle.abort();
        }
    }

    if !cfg!(windows) {
        let _ = tokio::fs::remove_file(&socket_path).await;
    }
    
    Ok(())
}

pub async fn spawn_ipc_writer(
    socket_path: String,
    mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) -> Result<()> {
    // Wait for socket
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    #[cfg(unix)]
    {
        if let Ok(mut stream) = tokio::net::UnixStream::connect(&socket_path).await {
            while let Some(cmd) = cmd_rx.recv().await {
                let _ = stream.write_all(cmd.as_bytes()).await;
                let _ = stream.flush().await;
            }
        }
    }

    #[cfg(windows)]
    {
        use tokio::net::windows::named_pipe::ClientOptions;
        if let Ok(mut client) = ClientOptions::new().open(&socket_path) {
            while let Some(cmd) = cmd_rx.recv().await {
                let _ = client.write_all(cmd.as_bytes()).await;
                let _ = client.flush().await;
            }
        }
    }

    Ok(())
}

