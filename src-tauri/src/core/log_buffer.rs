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
        let mut buf = LOG_BUFFER.lock().unwrap();

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
    let buf = LOG_BUFFER.lock().unwrap();
    let start = if buf.len() > count {
        buf.len() - count
    } else {
        0
    };
    buf[start..].to_vec()
}

pub fn clear_logs() {
    LOG_BUFFER.lock().unwrap().clear();
}
