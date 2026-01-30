use std::path::PathBuf;
use anyhow::Result;
use fern::colors::{Color, ColoredLevelConfig};
use std::sync::RwLock;
use std::fs::File;
use std::io::Write;
use std::sync::OnceLock;

// Global storage for the log file handle
static LOG_FILE: OnceLock<RwLock<Option<File>>> = OnceLock::new();

/// A writer that delegates to the current global log file
struct DynamicWriter;

impl Write for DynamicWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(lock) = LOG_FILE.get() {
            if let Ok(mut file_opt) = lock.write() {
                if let Some(file) = file_opt.as_mut() {
                    return file.write(buf);
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(lock) = LOG_FILE.get() {
            if let Ok(mut file_opt) = lock.write() {
                if let Some(file) = file_opt.as_mut() {
                    return file.flush();
                }
            }
        }
        Ok(())
    }
}

pub fn update_log_path(path: PathBuf) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    let lock = LOG_FILE.get_or_init(|| RwLock::new(None));
    if let Ok(mut file_opt) = lock.write() {
        *file_opt = Some(file);
    }
    
    Ok(())
}

pub fn init_logger(path: PathBuf, enabled: bool) -> Result<()> {
    // Set the initial log file
    update_log_path(path)?;

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    // Set the global logging level based on the initial config
    let level = if enabled {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Off
    };
    log::set_max_level(level);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}]   {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        // Always allow Info+ in the dispatch, control actual output via global max level
        .level(log::LevelFilter::Info) 
        .chain(Box::new(DynamicWriter) as Box<dyn Write + Send + 'static>)
        .apply()?;

    Ok(())
}
