use std::path::PathBuf;
use anyhow::Result;
use fern::colors::{Color, ColoredLevelConfig};

pub fn init_logger(path: PathBuf, enabled: bool) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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
        .chain(fern::log_file(path)?)
        .apply()?;

    Ok(())
}
