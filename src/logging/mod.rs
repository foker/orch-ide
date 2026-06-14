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

    // Capture panic messages + backtrace into the log so we can diagnose
    // crashes that happen inside Objective-C blocks (where stderr is silent).
    // SAFETY: called once at process startup before any threads are spawned.
    unsafe { std::env::set_var("RUST_BACKTRACE", "1"); }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };
        let bt = std::backtrace::Backtrace::force_capture();
        log(&format!(
            "!!! PANIC at {} !!!\n  payload: {}\n  thread: {:?}\n  backtrace:\n{}",
            location,
            payload,
            std::thread::current().name().unwrap_or("<unnamed>"),
            bt,
        ));
        default_hook(info);
    }));
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
        let us = elapsed.as_micros() as u64;
        metrics::record(&self.label, us);
        if elapsed.as_millis() > 5 {
            log(&format!("[PERF] {} took {}ms", self.label, elapsed.as_millis()));
        }
    }
}

pub mod metrics {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::OnceLock;
    use std::time::Instant;

    #[derive(Default, Clone)]
    pub struct Stat {
        pub count: u64,
        pub total_us: u64,
        pub max_us: u64,
    }

    struct State {
        per_label: HashMap<String, Stat>,
        window_start: Instant,
    }

    static STATE: OnceLock<Mutex<State>> = OnceLock::new();

    fn state() -> &'static Mutex<State> {
        STATE.get_or_init(|| Mutex::new(State {
            per_label: HashMap::new(),
            window_start: Instant::now(),
        }))
    }

    pub fn record(label: &str, duration_us: u64) {
        let Ok(mut s) = state().lock() else { return; };
        let e = s.per_label.entry(label.to_string()).or_default();
        e.count += 1;
        e.total_us = e.total_us.saturating_add(duration_us);
        if duration_us > e.max_us { e.max_us = duration_us; }
    }

    /// Drain accumulated stats and write a summary to the log.
    /// Intended to be called periodically (e.g. on Tick). `min_count` filters out
    /// labels that barely fire, so the log stays readable.
    pub fn flush(min_count: u64) {
        let Ok(mut s) = state().lock() else { return; };
        let window = s.window_start.elapsed();
        s.window_start = Instant::now();
        let mut entries: Vec<(String, Stat)> = s.per_label.drain().collect();
        drop(s);

        if entries.is_empty() { return; }
        entries.sort_by(|a, b| b.1.total_us.cmp(&a.1.total_us));
        let window_secs = window.as_secs_f64().max(0.001);
        super::log(&format!("[METRICS] window={:.1}s", window_secs));
        for (label, stat) in entries {
            if stat.count < min_count { continue; }
            let avg_ms = (stat.total_us as f64 / stat.count as f64) / 1000.0;
            let total_ms = stat.total_us as f64 / 1000.0;
            let rate = stat.count as f64 / window_secs;
            super::log(&format!(
                "  {:<28} n={:>5} {:>6.1}/s  total={:>7.1}ms  avg={:>6.2}ms  max={:>6.2}ms",
                label, stat.count, rate, total_ms, avg_ms, stat.max_us as f64 / 1000.0
            ));
        }
    }
}

#[macro_export]
macro_rules! app_log {
    ($($arg:tt)*) => {
        $crate::logging::log(&format!($($arg)*))
    };
}
