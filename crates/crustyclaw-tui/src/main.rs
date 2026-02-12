#![deny(unsafe_code)]

//! CrustyClaw TUI — interactive terminal control plane.

mod app;
mod keymap;
mod panels;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Tabs},
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use app::{App, Panel};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up log collector so the TUI can display logs
    let log_collector = crustyclaw_core::LogCollector::new(1000);
    let log_reader = log_collector.reader();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(log_collector)
        .init();

    // Load config (best-effort)
    let config_path = PathBuf::from("crustyclaw.toml");
    let config = if config_path.exists() {
        crustyclaw_config::AppConfig::load(&config_path)
            .unwrap_or_else(|_| crustyclaw_config::AppConfig::default())
    } else {
        crustyclaw_config::AppConfig::default()
    };

    tracing::info!("Starting CrustyClaw TUI");

    // Set up terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let mut app = App::new(config, log_reader);

    // Main event loop
    let result = run_loop(&mut terminal, &mut app);

    // Restore terminal (always, even on error)
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        app.tick();
        terminal.draw(|frame| render(frame, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let action = app.keymap.resolve(key.code);
                    app.handle_action(action);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + tabs
            Constraint::Min(1),    // main content
            Constraint::Length(2), // status bar
        ])
        .split(frame.area());

    // Header with tab bar
    render_header(frame, app, chunks[0]);

    // Main panel content
    match app.active_panel {
        Panel::Dashboard => app.dashboard.render(frame, chunks[1]),
        Panel::Logs => app.logs.render(frame, chunks[1]),
        Panel::Messages => app.messages.render(frame, chunks[1]),
        Panel::Config => app.config_panel.render(frame, chunks[1]),
    }

    // Status bar
    let status = Paragraph::new(app.status_line()).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ["1:Dashboard", "2:Logs", "3:Messages", "4:Config"]
        .iter()
        .map(|t| Line::from(*t))
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().title(" CrustyClaw ").borders(Borders::ALL))
        .select(app.active_panel.index())
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    frame.render_widget(tabs, area);
}
