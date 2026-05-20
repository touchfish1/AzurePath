use std::sync::LazyLock;
use std::sync::Mutex;
use tracing_subscriber::Layer;

static LOG_BUFFER: LazyLock<Mutex<Vec<LogEntry>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub struct LogLayer;

impl<S: tracing::Subscriber> Layer<S> for LogLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut buf = LOG_BUFFER.lock().unwrap_or_else(|e| e.into_inner());

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let level = format!("{}", event.metadata().level());
        let target = event.metadata().target().to_string();

        // Collect all fields
        let mut message = String::new();
        let mut visitor = LogVisitor(&mut message);
        event.record(&mut visitor);

        buf.push(LogEntry {
            timestamp,
            level,
            target,
            message,
        });

        // Keep only last 1000 entries
        if buf.len() > 1000 {
            let excess = buf.len() - 1000;
            buf.drain(0..excess);
        }
    }
}

struct LogVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for LogVisitor<'a> {
    fn record_debug(
        &mut self,
        field: &tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        let formatted = format!("{:?}", value);
        if field.name() == "message" {
            // Strip surrounding quotes added by Debug formatting for cleaner messages
            if formatted.starts_with('"')
                && formatted.ends_with('"')
                && formatted.len() >= 2
            {
                self.0.push_str(&formatted[1..formatted.len() - 1]);
            } else {
                self.0.push_str(&formatted);
            }
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0
                .push_str(&format!("{}={}", field.name(), formatted));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            if !self.0.is_empty() {
                self.0.push(' ');
            }
            self.0.push_str(&format!("{}={}", field.name(), value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(&format!("{}={}", field.name(), value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push_str(&format!("{}={}", field.name(), value));
    }
}

pub fn get_logs(count: usize) -> Vec<LogEntry> {
    let buf = LOG_BUFFER.lock().unwrap_or_else(|e| e.into_inner());
    let start = if buf.len() > count {
        buf.len() - count
    } else {
        0
    };
    buf[start..].to_vec()
}

pub fn clear_logs() {
    LOG_BUFFER.lock().unwrap_or_else(|e| e.into_inner()).clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tracing_subscriber::prelude::*;

    /// Serializes all log_buffer tests since they access the global LOG_BUFFER.
    static LOG_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Set up LogLayer as the current thread's subscriber and clear the buffer.
    fn setup() -> (tracing::subscriber::DefaultGuard, std::sync::MutexGuard<'static, ()>) {
        let lock = LOG_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_logs();
        let subscriber = tracing_subscriber::Registry::default().with(LogLayer);
        let guard = tracing::subscriber::set_default(subscriber);
        (guard, lock)
    }

    // ── Level tests ────────────────────────────────────────────────

    #[test]
    fn test_info_log() {
        let (_sg, _lg) = setup();
        tracing::info!("info message");
        let logs = get_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, "INFO");
        assert_eq!(logs[0].message, "info message");
    }

    #[test]
    fn test_warn_log() {
        let (_sg, _lg) = setup();
        tracing::warn!("warn message");
        let logs = get_logs(10);
        assert_eq!(logs[0].level, "WARN");
        assert_eq!(logs[0].message, "warn message");
    }

    #[test]
    fn test_error_log() {
        let (_sg, _lg) = setup();
        tracing::error!("error message");
        let logs = get_logs(10);
        assert_eq!(logs[0].level, "ERROR");
        assert_eq!(logs[0].message, "error message");
    }

    #[test]
    fn test_debug_log() {
        let (_sg, _lg) = setup();
        tracing::debug!("debug message");
        let logs = get_logs(10);
        assert_eq!(logs[0].level, "DEBUG");
        assert_eq!(logs[0].message, "debug message");
    }

    #[test]
    fn test_trace_log() {
        let (_sg, _lg) = setup();
        tracing::trace!("trace message");
        let logs = get_logs(10);
        assert_eq!(logs[0].level, "TRACE");
        assert_eq!(logs[0].message, "trace message");
    }

    // ── Order and count tests ──────────────────────────────────────

    #[test]
    fn test_multiple_logs_in_order() {
        let (_sg, _lg) = setup();
        tracing::info!("first");
        tracing::warn!("second");
        tracing::error!("third");
        let logs = get_logs(10);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message, "first");
        assert_eq!(logs[1].message, "second");
        assert_eq!(logs[2].message, "third");
    }

    #[test]
    fn test_get_logs_count_returns_newest_n() {
        let (_sg, _lg) = setup();
        for i in 0..10 {
            tracing::info!("msg {}", i);
        }
        let logs = get_logs(3);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message, "msg 7");
        assert_eq!(logs[1].message, "msg 8");
        assert_eq!(logs[2].message, "msg 9");
    }

    #[test]
    fn test_get_logs_zero_returns_empty() {
        let (_sg, _lg) = setup();
        tracing::info!("only one");
        let logs = get_logs(0);
        assert!(logs.is_empty());
    }

    #[test]
    fn test_get_logs_more_than_available() {
        let (_sg, _lg) = setup();
        tracing::info!("only one");
        let logs = get_logs(100);
        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_get_logs_empty_buffer() {
        let (_sg, _lg) = setup();
        let logs = get_logs(10);
        assert!(logs.is_empty());
    }

    // ── Field tests ────────────────────────────────────────────────

    #[test]
    fn test_log_has_timestamp() {
        let (_sg, _lg) = setup();
        tracing::info!("timed");
        let logs = get_logs(10);
        assert!(!logs[0].timestamp.is_empty(), "timestamp should not be empty");
        assert!(logs[0].timestamp.contains('T'), "timestamp should be ISO 8601: {}", logs[0].timestamp);
    }

    #[test]
    fn test_log_has_target() {
        let (_sg, _lg) = setup();
        tracing::info!("targeted");
        let logs = get_logs(10);
        assert!(!logs[0].target.is_empty(), "target should not be empty");
    }

    #[test]
    fn test_log_extra_field_debug() {
        let (_sg, _lg) = setup();
        tracing::info!(custom_field = "custom_value", "msg with extra");
        let logs = get_logs(10);
        assert_eq!(logs.len(), 1);
        assert!(
            logs[0].message.contains("custom_field=custom_value")
                || logs[0].message.contains("custom_field=\"custom_value\""),
            "extra field should appear in message: {:?}",
            logs[0].message
        );
    }

    #[test]
    fn test_log_field_i64() {
        let (_sg, _lg) = setup();
        tracing::info!(count = 42, "int field");
        let logs = get_logs(10);
        assert!(logs[0].message.contains("count=42"));
    }

    #[test]
    fn test_log_field_bool() {
        let (_sg, _lg) = setup();
        tracing::info!(flag = true, "bool field");
        let logs = get_logs(10);
        assert!(logs[0].message.contains("flag=true"));
    }

    // ── clear_logs tests ───────────────────────────────────────────

    #[test]
    fn test_clear_logs_empty_buffer() {
        let (_sg, _lg) = setup();
        clear_logs();
        assert!(get_logs(10).is_empty());
    }

    #[test]
    fn test_clear_logs_after_logging() {
        let (_sg, _lg) = setup();
        tracing::info!("to be cleared");
        assert_eq!(get_logs(10).len(), 1);
        clear_logs();
        assert!(get_logs(10).is_empty());
    }

    #[test]
    fn test_log_after_clear() {
        let (_sg, _lg) = setup();
        tracing::info!("before");
        clear_logs();
        tracing::info!("after");
        let logs = get_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "after");
    }

    #[test]
    fn test_clear_logs_twice() {
        let (_sg, _lg) = setup();
        clear_logs();
        clear_logs();
        assert!(get_logs(10).is_empty());
    }

    // ── Ring buffer overflow ───────────────────────────────────────

    #[test]
    fn test_buffer_overflow_drops_oldest() {
        let (_sg, _lg) = setup();
        for i in 0..1010u32 {
            tracing::info!("overflow {}", i);
        }
        let logs = get_logs(2000);
        assert_eq!(logs.len(), 1000, "buffer should cap at 1000 entries");
        // Oldest 10 entries should be dropped, so first kept is "overflow 10"
        assert_eq!(logs[0].message, "overflow 10");
        assert_eq!(logs[logs.len() - 1].message, "overflow 1009");
    }

    #[test]
    fn test_buffer_exactly_1000() {
        let (_sg, _lg) = setup();
        for i in 0..1000u32 {
            tracing::info!("exact {}", i);
        }
        let logs = get_logs(2000);
        assert_eq!(logs.len(), 1000);
        assert_eq!(logs[0].message, "exact 0");
        assert_eq!(logs[999].message, "exact 999");
    }

    #[test]
    fn test_buffer_at_boundary_1001() {
        let (_sg, _lg) = setup();
        for i in 0..1001u32 {
            tracing::info!("boundary {}", i);
        }
        let logs = get_logs(2000);
        assert_eq!(logs.len(), 1000);
        assert_eq!(logs[0].message, "boundary 1");
        assert_eq!(logs[999].message, "boundary 1000");
    }

    // ── Edge cases ─────────────────────────────────────────────────

    #[test]
    fn test_empty_message() {
        let (_sg, _lg) = setup();
        tracing::info!("");
        let logs = get_logs(10);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "");
    }

    #[test]
    fn test_message_with_newlines() {
        let (_sg, _lg) = setup();
        tracing::info!("line1\nline2");
        let logs = get_logs(10);
        assert_eq!(logs[0].message, "line1\nline2");
    }

    #[test]
    fn test_unicode_message() {
        let (_sg, _lg) = setup();
        tracing::info!("你好世界 🎉");
        let logs = get_logs(10);
        assert_eq!(logs[0].message, "你好世界 🎉");
    }

    #[test]
    fn test_message_with_special_chars() {
        let (_sg, _lg) = setup();
        tracing::info!("special: \"quoted\" & <tag>");
        let logs = get_logs(10);
        // The message should preserve special characters (with or without Debug quoting)
        assert!(
            logs[0].message.contains("quoted") || logs[0].message.contains("special"),
            "message should contain content: {:?}",
            logs[0].message
        );
    }
}
