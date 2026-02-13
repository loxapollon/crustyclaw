//! Config panel — display current configuration as formatted TOML.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::PanelState;

/// Configuration viewer panel with TOML syntax highlighting.
pub struct ConfigPanel {
    /// The rendered TOML text (split into lines).
    lines: Vec<String>,
    /// Scroll offset from top.
    scroll_offset: usize,
}

impl ConfigPanel {
    pub fn new(toml_text: String) -> Self {
        let lines: Vec<String> = toml_text.lines().map(String::from).collect();
        Self {
            lines,
            scroll_offset: 0,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize;

        let visible_lines: Vec<Line> = self
            .lines
            .iter()
            .skip(self.scroll_offset)
            .take(visible_height)
            .map(|line| {
                // Colorize TOML sections and keys
                if line.starts_with('[') {
                    Line::from(Span::styled(
                        line.as_str(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else if let Some((key, value)) = line.split_once(" = ") {
                    Line::from(vec![
                        Span::styled(key, Style::default().fg(Color::Yellow)),
                        Span::raw(" = "),
                        Span::styled(value, Style::default().fg(Color::Green)),
                    ])
                } else {
                    Line::from(line.as_str())
                }
            })
            .collect();

        let total = self.lines.len();
        let title = format!(" Config ({total} lines) ");

        let paragraph = Paragraph::new(visible_lines)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    }
}

impl PanelState for ConfigPanel {
    fn scroll_down(&mut self, n: usize) {
        let max = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + n).min(max);
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.lines.len().saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_panel_lines() {
        let toml = "[daemon]\nlisten_addr = \"127.0.0.1\"\nlisten_port = 9100\n";
        let panel = ConfigPanel::new(toml.to_string());
        assert_eq!(panel.lines.len(), 3);
    }

    #[test]
    fn test_config_panel_scroll() {
        let toml = (0..20)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut panel = ConfigPanel::new(toml);
        assert_eq!(panel.scroll_offset, 0);

        panel.scroll_down(5);
        assert_eq!(panel.scroll_offset, 5);

        panel.scroll_up(3);
        assert_eq!(panel.scroll_offset, 2);

        panel.scroll_to_top();
        assert_eq!(panel.scroll_offset, 0);

        panel.scroll_to_bottom();
        assert_eq!(panel.scroll_offset, 19);
    }
}
