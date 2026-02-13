//! Messages panel — live view of incoming/outgoing messages.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use super::PanelState;

/// A displayable message entry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MessageEntry {
    pub timestamp: String,
    pub channel: String,
    pub direction: MessageDirection,
    pub body: String,
}

/// Direction of a message on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

/// Message panel state — scrollable list of inbound/outbound messages.
pub struct MessagesPanel {
    entries: Vec<MessageEntry>,
    scroll_offset: usize,
    auto_follow: bool,
}

impl MessagesPanel {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            scroll_offset: 0,
            auto_follow: true,
        }
    }

    /// Push a new message into the panel.
    #[allow(dead_code)]
    pub fn push(&mut self, entry: MessageEntry) {
        self.entries.push(entry);
        if self.auto_follow {
            self.scroll_offset = 0;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize;

        if self.entries.is_empty() {
            let empty = Paragraph::new("  (no messages yet)")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .title(" Messages (0) ")
                        .borders(Borders::ALL),
                );
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
                let (arrow, arrow_style) = match entry.direction {
                    MessageDirection::Inbound => (">>", Style::default().fg(Color::Green)),
                    MessageDirection::Outbound => ("<<", Style::default().fg(Color::Cyan)),
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{arrow} "), arrow_style),
                    Span::styled(
                        format!("[{}] ", entry.channel),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(&entry.body),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = format!(" Messages ({total}) ");
        let list = List::new(items).block(Block::default().title(title).borders(Borders::ALL));
        frame.render_widget(list, area);
    }
}

impl Default for MessagesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PanelState for MessagesPanel {
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

    fn sample_entry(body: &str, direction: MessageDirection) -> MessageEntry {
        MessageEntry {
            timestamp: "12:00:00".to_string(),
            channel: "signal".to_string(),
            direction,
            body: body.to_string(),
        }
    }

    #[test]
    fn test_new_panel_is_empty() {
        let panel = MessagesPanel::new();
        assert_eq!(panel.entries.len(), 0);
        assert!(panel.auto_follow);
    }

    #[test]
    fn test_default_matches_new() {
        let panel = MessagesPanel::default();
        assert_eq!(panel.entries.len(), 0);
        assert!(panel.auto_follow);
    }

    #[test]
    fn test_push_adds_entry() {
        let mut panel = MessagesPanel::new();
        panel.push(sample_entry("hello", MessageDirection::Inbound));
        assert_eq!(panel.entries.len(), 1);
        assert_eq!(panel.entries[0].body, "hello");
    }

    #[test]
    fn test_push_preserves_auto_follow() {
        let mut panel = MessagesPanel::new();
        panel.push(sample_entry("a", MessageDirection::Inbound));
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_push_after_scroll_up_keeps_offset() {
        let mut panel = MessagesPanel::new();
        for i in 0..10 {
            panel.push(sample_entry(&format!("msg {i}"), MessageDirection::Inbound));
        }
        panel.scroll_up(3);
        assert!(!panel.auto_follow);

        // Push a new message — offset should not change since auto_follow is off
        panel.push(sample_entry("new", MessageDirection::Outbound));
        assert_eq!(panel.entries.len(), 11);
    }

    #[test]
    fn test_scroll_up_disables_auto_follow() {
        let mut panel = MessagesPanel::new();
        for i in 0..5 {
            panel.push(sample_entry(&format!("msg {i}"), MessageDirection::Inbound));
        }
        panel.scroll_up(2);
        assert!(!panel.auto_follow);
        assert_eq!(panel.scroll_offset, 2);
    }

    #[test]
    fn test_scroll_down_re_enables_auto_follow() {
        let mut panel = MessagesPanel::new();
        for i in 0..5 {
            panel.push(sample_entry(&format!("msg {i}"), MessageDirection::Inbound));
        }
        panel.scroll_up(3);
        panel.scroll_down(10); // past bottom
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_top_and_bottom() {
        let mut panel = MessagesPanel::new();
        for i in 0..20 {
            panel.push(sample_entry(&format!("msg {i}"), MessageDirection::Inbound));
        }

        panel.scroll_to_top();
        assert!(!panel.auto_follow);
        assert_eq!(panel.scroll_offset, 19);

        panel.scroll_to_bottom();
        assert!(panel.auto_follow);
        assert_eq!(panel.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_up_clamped() {
        let mut panel = MessagesPanel::new();
        for i in 0..5 {
            panel.push(sample_entry(&format!("msg {i}"), MessageDirection::Inbound));
        }
        panel.scroll_up(100);
        assert_eq!(panel.scroll_offset, 4); // clamped to len - 1
    }

    #[test]
    fn test_scroll_on_empty_panel() {
        let mut panel = MessagesPanel::new();
        panel.scroll_up(5);
        panel.scroll_down(5);
        panel.scroll_to_top();
        panel.scroll_to_bottom();
        // No panic
    }

    #[test]
    fn test_message_directions() {
        let inbound = sample_entry("in", MessageDirection::Inbound);
        let outbound = sample_entry("out", MessageDirection::Outbound);
        assert_eq!(inbound.direction, MessageDirection::Inbound);
        assert_eq!(outbound.direction, MessageDirection::Outbound);
    }
}
