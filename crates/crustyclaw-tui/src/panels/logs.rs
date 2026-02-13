//! Logs panel — scrollable live log viewer.

use crustyclaw_core::LogReader;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};
use tracing::Level;

use super::PanelState;

/// Scrollable log viewer panel with auto-follow.
pub struct LogsPanel {
    reader: LogReader,
    /// Cached snapshot of log entries (refreshed on tick).
    entries: Vec<LogLine>,
    /// Scroll offset (0 = bottom/latest).
    scroll_offset: usize,
    /// Whether to auto-follow (stick to bottom).
    auto_follow: bool,
}

struct LogLine {
    elapsed: String,
    level: Level,
    target: String,
    message: String,
}

impl LogsPanel {
    pub fn new(reader: LogReader) -> Self {
        Self {
            reader,
            entries: Vec::new(),
            scroll_offset: 0,
            auto_follow: true,
        }
    }

    /// Refresh cached entries from the log reader.
    pub fn refresh(&mut self) {
        self.entries = self
            .reader
            .entries()
            .into_iter()
            .map(|e| LogLine {
                elapsed: format!("{:>8.2}s", e.elapsed_secs),
                level: e.level,
                target: e.target,
                message: e.message,
            })
            .collect();

        // If auto-following, keep scroll at bottom
        if self.auto_follow {
            self.scroll_offset = 0;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // minus borders

        if self.entries.is_empty() {
            let empty = ratatui::widgets::Paragraph::new("  (no log entries yet)")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().title(" Logs (0) ").borders(Borders::ALL));
            frame.render_widget(empty, area);
            return;
        }

        let total = self.entries.len();
        let skip = if total > visible_height + self.scroll_offset {
            total - visible_height - self.scroll_offset
        } else {
            0
        };

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .skip(skip)
            .take(visible_height)
            .map(|entry| {
                let level_style = match entry.level {
                    Level::ERROR => Style::default().fg(Color::Red),
                    Level::WARN => Style::default().fg(Color::Yellow),
                    Level::INFO => Style::default().fg(Color::Green),
                    Level::DEBUG => Style::default().fg(Color::Blue),
                    Level::TRACE => Style::default().fg(Color::DarkGray),
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", entry.elapsed),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{:>5} ", entry.level), level_style),
                    Span::styled(
                        format!("{}: ", entry.target),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(&entry.message),
                ]);
                ListItem::new(line)
            })
            .collect();

        let follow_indicator = if self.auto_follow { " [follow]" } else { "" };
        let title = format!(" Logs ({total}){follow_indicator} ");

        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
        frame.render_widget(list, area);
    }
}

impl PanelState for LogsPanel {
    fn scroll_down(&mut self, n: usize) {
        if self.scroll_offset >= n {
            self.scroll_offset -= n;
        } else {
            self.scroll_offset = 0;
            self.auto_follow = true;
        }
    }

    fn scroll_up(&mut self, n: usize) {
        self.auto_follow = false;
        let max_offset = self.entries.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }

    fn scroll_to_top(&mut self) {
        self.auto_follow = false;
        self.scroll_offset = self.entries.len().saturating_sub(1);
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_follow = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    fn make_logs_panel_with_entries(count: usize) -> LogsPanel {
        let collector = crustyclaw_core::LogCollector::new(1000);
        let reader = collector.reader();

        // Emit tracing events so the collector captures them
        let subscriber = tracing_subscriber::registry().with(collector);
        let _guard = tracing::subscriber::set_default(subscriber);
        for i in 0..count {
            tracing::info!("test log entry {i}");
        }

        let mut panel = LogsPanel::new(reader);
        panel.refresh();
        panel
    }

    #[test]
    fn test_new_panel_starts_empty() {
        let collector = crustyclaw_core::LogCollector::new(100);
        let panel = LogsPanel::new(collector.reader());
        assert_eq!(panel.entries.len(), 0);
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_refresh_captures_entries() {
        let panel = make_logs_panel_with_entries(5);
        assert_eq!(panel.entries.len(), 5);
    }

    #[test]
    fn test_auto_follow_enabled_by_default() {
        let panel = make_logs_panel_with_entries(3);
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_disables_auto_follow() {
        let mut panel = make_logs_panel_with_entries(20);
        assert!(panel.auto_follow);

        panel.scroll_up(5);
        assert!(!panel.auto_follow);
        assert_eq!(panel.scroll_offset, 5);
    }

    #[test]
    fn test_scroll_down_re_enables_auto_follow_at_bottom() {
        let mut panel = make_logs_panel_with_entries(20);

        panel.scroll_up(3);
        assert!(!panel.auto_follow);

        // Scroll down past bottom
        panel.scroll_down(10);
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut panel = make_logs_panel_with_entries(20);
        panel.scroll_to_top();
        assert!(!panel.auto_follow);
        assert_eq!(panel.scroll_offset, 19); // len - 1
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut panel = make_logs_panel_with_entries(20);
        panel.scroll_up(10);
        panel.scroll_to_bottom();
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_clamped_to_max() {
        let mut panel = make_logs_panel_with_entries(5);
        panel.scroll_up(100);
        assert_eq!(panel.scroll_offset, 4); // clamped to len - 1
    }

    #[test]
    fn test_scroll_on_empty_panel() {
        let collector = crustyclaw_core::LogCollector::new(100);
        let mut panel = LogsPanel::new(collector.reader());
        // Should not panic
        panel.scroll_up(5);
        panel.scroll_down(5);
        panel.scroll_to_top();
        panel.scroll_to_bottom();
    }
}
