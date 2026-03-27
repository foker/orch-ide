use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;
use std::sync::Mutex;

static LOG_FILE: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

const LOG_PATH: &str = "/tmp/claude-sessions-debug.log";

pub fn init() {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
        .expect("Failed to open log file");
    LOG_FILE.set(Mutex::new(file)).ok();
    log("=== Claude Sessions started ===");
}

pub fn log(msg: &str) {
    if let Some(file) = LOG_FILE.get() {
        if let Ok(mut f) = file.lock() {
            let now = chrono::Local::now().format("%H:%M:%S%.3f");
            writeln!(f, "[{}] {}", now, msg).ok();
            f.flush().ok();
        }
    }
}

/// Log with duration measurement. Returns a guard that logs elapsed time on drop.
pub fn perf(label: &str) -> PerfGuard {
    PerfGuard {
        label: label.to_string(),
        start: std::time::Instant::now(),
    }
}

pub struct PerfGuard {
    label: String,
    start: std::time::Instant,
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        if elapsed.as_millis() > 50 {
            // Only log slow operations (>50ms)
            log(&format!("[PERF] {} took {}ms", self.label, elapsed.as_millis()));
        }
    }
}

#[macro_export]
macro_rules! app_log {
    ($($arg:tt)*) => {
        $crate::logging::log(&format!($($arg)*))
    };
}
