//! Logs panel — scrollable live log viewer.

use crustyclaw_core::LogReader;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem},
};
use tracing::Level;

use super::PanelState;

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
