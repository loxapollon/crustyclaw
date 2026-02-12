//! In-memory log collector for the TUI.
//!
//! Provides a [`LogCollector`] that captures `tracing` events into a bounded
//! ring buffer, and a [`LogReader`] handle for reading captured entries.

use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// A single captured log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp as seconds since the collector was created.
    pub elapsed_secs: f64,
    /// Log level.
    pub level: Level,
    /// Target module path.
    pub target: String,
    /// The formatted message.
    pub message: String,
}

/// Shared buffer backing the log collector.
#[derive(Debug)]
struct LogBuffer {
    entries: Vec<LogEntry>,
    capacity: usize,
    start_time: std::time::Instant,
}

impl LogBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
            start_time: std::time::Instant::now(),
        }
    }

    fn push(&mut self, level: Level, target: String, message: String) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(LogEntry {
            elapsed_secs: self.start_time.elapsed().as_secs_f64(),
            level,
            target,
            message,
        });
    }
}

/// A `tracing` layer that captures log events into a shared ring buffer.
///
/// Attach this to a `tracing_subscriber` registry so that all log events
/// are available to the TUI log panel.
#[derive(Debug, Clone)]
pub struct LogCollector {
    buffer: Arc<Mutex<LogBuffer>>,
}

impl LogCollector {
    /// Create a new collector with the given ring buffer capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(LogBuffer::new(capacity))),
        }
    }

    /// Get a reader handle for the captured log entries.
    pub fn reader(&self) -> LogReader {
        LogReader {
            buffer: Arc::clone(&self.buffer),
        }
    }
}

impl<S: Subscriber> Layer<S> for LogCollector {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();
        let target = metadata.target().to_string();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(level, target, visitor.message);
        }
    }
}

/// A read handle for the log buffer.
#[derive(Debug, Clone)]
pub struct LogReader {
    buffer: Arc<Mutex<LogBuffer>>,
}

impl LogReader {
    /// Return a snapshot of all captured log entries.
    pub fn entries(&self) -> Vec<LogEntry> {
        self.buffer
            .lock()
            .map(|buf| buf.entries.clone())
            .unwrap_or_default()
    }

    /// Return the number of entries currently in the buffer.
    pub fn len(&self) -> usize {
        self.buffer.lock().map(|buf| buf.entries.len()).unwrap_or(0)
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Visitor that extracts the `message` field from a tracing event.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    #[test]
    fn test_log_collector_captures_events() {
        let collector = LogCollector::new(100);
        let reader = collector.reader();

        let _guard = tracing_subscriber::registry().with(collector).set_default();

        tracing::info!("hello from test");
        tracing::warn!("a warning");

        let entries = reader.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, Level::INFO);
        assert_eq!(entries[1].level, Level::WARN);
    }

    #[test]
    fn test_log_collector_ring_buffer() {
        let collector = LogCollector::new(3);
        let reader = collector.reader();

        let _guard = tracing_subscriber::registry().with(collector).set_default();

        tracing::info!("one");
        tracing::info!("two");
        tracing::info!("three");
        tracing::info!("four");

        let entries = reader.entries();
        assert_eq!(entries.len(), 3);
        // First entry ("one") should have been evicted
        assert!(entries[0].message.contains("two"));
    }

    #[test]
    fn test_log_reader_is_empty() {
        let collector = LogCollector::new(10);
        let reader = collector.reader();
        assert!(reader.is_empty());
        assert_eq!(reader.len(), 0);
    }
}
