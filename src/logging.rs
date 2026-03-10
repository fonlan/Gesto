use std::{
    fs::{self, OpenOptions},
    io::Write,
    panic,
    path::PathBuf,
    thread,
};

use anyhow::{Context, anyhow};
use chrono::Local;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;

struct Logger {
    dir: PathBuf,
    lock: Mutex<()>,
}

static LOGGER: OnceCell<Logger> = OnceCell::new();

pub fn init(dir: PathBuf) -> anyhow::Result<()> {
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create logs directory: {}", dir.display()))?;
    LOGGER
        .set(Logger {
            dir: dir.clone(),
            lock: Mutex::new(()),
        })
        .map_err(|_| anyhow!("logger already initialized"))?;
    info(format!("logger initialized at {}", dir.display()));
    Ok(())
}

pub fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let payload = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            (*message).to_string()
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            message.clone()
        } else {
            "panic payload unavailable".to_string()
        };
        error(format!("panic at {}: {}", location, payload));
        default_hook(panic_info);
    }));
}

pub fn info(message: impl AsRef<str>) {
    write("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write("ERROR", message.as_ref());
}

fn write(level: &str, message: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };

    let _guard = logger.lock.lock();
    let _ = write_lines(&logger.dir, level, message);
}

fn write_lines(dir: &PathBuf, level: &str, message: &str) -> anyhow::Result<()> {
    let now = Local::now();
    let file_name = format!("{}.log", now.format("%Y-%m-%d"));
    let path = dir.join(file_name);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open log file: {}", path.display()))?;
    let current_thread = thread::current();
    let thread_name = current_thread.name().unwrap_or("unnamed");

    for line in message.lines() {
        writeln!(
            file,
            "{} [{}] [{}] {}",
            now.format("%Y-%m-%d %H:%M:%S%.3f"),
            level,
            thread_name,
            line
        )?;
    }

    if message.is_empty() {
        writeln!(
            file,
            "{} [{}] [{}]",
            now.format("%Y-%m-%d %H:%M:%S%.3f"),
            level,
            thread_name
        )?;
    }

    file.flush()?;
    Ok(())
}
