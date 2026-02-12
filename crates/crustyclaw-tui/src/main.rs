#![deny(unsafe_code)]

//! CrustyClaw TUI — interactive terminal control plane.

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use tracing::info;

/// TUI application state.
struct App {
    /// Whether the application should quit.
    should_quit: bool,

    /// Currently selected panel.
    active_panel: Panel,

    /// Status message displayed at the bottom.
    status_message: String,
}

/// The panels available in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Dashboard,
    Logs,
    Messages,
    Config,
}

impl Panel {
    fn title(self) -> &'static str {
        match self {
            Panel::Dashboard => "Dashboard",
            Panel::Logs => "Logs",
            Panel::Messages => "Messages",
            Panel::Config => "Config",
        }
    }

    fn next(self) -> Self {
        match self {
            Panel::Dashboard => Panel::Logs,
            Panel::Logs => Panel::Messages,
            Panel::Messages => Panel::Config,
            Panel::Config => Panel::Dashboard,
        }
    }

    fn prev(self) -> Self {
        match self {
            Panel::Dashboard => Panel::Config,
            Panel::Logs => Panel::Dashboard,
            Panel::Messages => Panel::Logs,
            Panel::Config => Panel::Messages,
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            should_quit: false,
            active_panel: Panel::Dashboard,
            status_message: "Press 'q' to quit, Tab/Shift+Tab to switch panels".to_string(),
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => self.active_panel = self.active_panel.next(),
            KeyCode::BackTab => self.active_panel = self.active_panel.prev(),
            KeyCode::Char('1') => self.active_panel = Panel::Dashboard,
            KeyCode::Char('2') => self.active_panel = Panel::Logs,
            KeyCode::Char('3') => self.active_panel = Panel::Messages,
            KeyCode::Char('4') => self.active_panel = Panel::Config,
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // header
                Constraint::Min(1),    // main content
                Constraint::Length(3), // status bar
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new("CrustyClaw TUI Control Plane")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(header, chunks[0]);

        // Main content panel
        let panel_title = format!(" {} ", self.active_panel.title());
        let content = match self.active_panel {
            Panel::Dashboard => "Daemon status: not connected\nUptime: —\nChannels: 0",
            Panel::Logs => "(No log entries)",
            Panel::Messages => "(No messages)",
            Panel::Config => "(Configuration view pending)",
        };
        let panel = Paragraph::new(content)
            .block(Block::default().title(panel_title).borders(Borders::ALL));
        frame.render_widget(panel, chunks[1]);

        // Status bar
        let status = Paragraph::new(self.status_message.as_str())
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(status, chunks[2]);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load config (best-effort)
    let config_path = PathBuf::from("crustyclaw.toml");
    let _config = if config_path.exists() {
        crustyclaw_config::AppConfig::load(&config_path)
            .unwrap_or_else(|_| crustyclaw_config::AppConfig::default())
    } else {
        crustyclaw_config::AppConfig::default()
    };

    info!("Starting CrustyClaw TUI");

    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new();

    // Main event loop
    while !app.should_quit {
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
