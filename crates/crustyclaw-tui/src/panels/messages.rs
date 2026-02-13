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
