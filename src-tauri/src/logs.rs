//! A logger that keeps recent lines for the Logs page and writes a file.
//! lifter-core logs through the same `log` macros, so its messages show up
//! here too.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Line {
    pub time: String,
    pub level: String,
    pub message: String,
}

pub struct Logger {
    lines: Mutex<VecDeque<Line>>,
    file: Mutex<Option<std::fs::File>>,
}

const KEEP: usize = 3000;

pub static LOGGER: std::sync::LazyLock<Logger> = std::sync::LazyLock::new(|| {
    let path = crate::store::data_dir().join("obtainit.log");
    let _ = std::fs::create_dir_all(crate::store::data_dir());
    if std::fs::metadata(&path).map(|m| m.len() > 4 << 20).unwrap_or(false) {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    let file = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok();
    Logger { lines: Mutex::new(VecDeque::new()), file: Mutex::new(file) }
});

impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        let t = m.target();
        if t.starts_with("obtainit") || t.starts_with("lifter_core") {
            m.level() <= log::Level::Debug
        } else {
            m.level() <= log::Level::Warn
        }
    }

    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = Line {
            time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level: r.level().to_string(),
            message: r.args().to_string(),
        };
        let text = format!("{} {:<5} {}\n", line.time, line.level, line.message);
        if r.level() <= log::Level::Info {
            eprint!("{text}");
            if let Ok(mut f) = self.file.lock() {
                if let Some(f) = f.as_mut() {
                    let _ = f.write_all(text.as_bytes());
                }
            }
        }
        let mut l = self.lines.lock().unwrap();
        if l.len() >= KEEP {
            l.pop_front();
        }
        l.push_back(line);
    }

    fn flush(&self) {}
}

pub fn init() {
    let _ = log::set_logger(&*LOGGER);
    log::set_max_level(log::LevelFilter::Debug);
}

pub fn lines() -> Vec<Line> {
    LOGGER.lines.lock().unwrap().iter().cloned().collect()
}

pub fn clear() {
    LOGGER.lines.lock().unwrap().clear();
}
