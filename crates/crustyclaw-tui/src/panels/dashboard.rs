//! Dashboard panel — daemon status, uptime, channel info.

use std::time::Duration;

use crustyclaw_config::AppConfig;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use super::PanelState;

/// Dashboard panel state — shows daemon uptime, listen address, channel status.
pub struct DashboardPanel {
    pub uptime: Duration,
    pub listen_addr: String,
    pub listen_port: u16,
    pub signal_enabled: bool,
    pub log_level: String,
    pub scroll_offset: usize,
}

impl DashboardPanel {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            uptime: Duration::ZERO,
            listen_addr: config.daemon.listen_addr.clone(),
            listen_port: config.daemon.listen_port,
            signal_enabled: config.signal.enabled,
            log_level: config.logging.level.clone(),
            scroll_offset: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(0)])
            .split(area);

        // Status block
        let uptime = format_duration(self.uptime);
        let status_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled("running", Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("Uptime: ", Style::default().fg(Color::Gray)),
                Span::raw(&uptime),
            ]),
            Line::from(vec![
                Span::styled("Listen: ", Style::default().fg(Color::Gray)),
                Span::raw(format!("{}:{}", self.listen_addr, self.listen_port)),
            ]),
        ];
        let status = Paragraph::new(status_text)
            .block(Block::default().title(" Status ").borders(Borders::ALL));
        frame.render_widget(status, chunks[0]);

        // Channels table
        let signal_status = if self.signal_enabled {
            Span::styled("enabled", Style::default().fg(Color::Green))
        } else {
            Span::styled("disabled", Style::default().fg(Color::DarkGray))
        };

        let rows = vec![
            Row::new(vec![Cell::from("Signal"), Cell::from(signal_status)]),
            Row::new(vec![
                Cell::from("Log level"),
                Cell::from(self.log_level.as_str()),
            ]),
        ];

        let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(10)])
            .header(
                Row::new(vec!["Channel", "Status"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().title(" Channels ").borders(Borders::ALL));
        frame.render_widget(table, chunks[1]);
    }
}

impl PanelState for DashboardPanel {
    fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }
    fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }
    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }
    fn scroll_to_bottom(&mut self) {
        // Dashboard is small, no-op
    }
}

fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00:00");
        assert_eq!(format_duration(Duration::from_secs(61)), "00:01:01");
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01");
    }

    #[test]
    fn test_dashboard_from_config() {
        let config = AppConfig::default();
        let panel = DashboardPanel::new(&config);
        assert_eq!(panel.listen_addr, "127.0.0.1");
        assert_eq!(panel.listen_port, 9100);
        assert!(!panel.signal_enabled);
    }
}
