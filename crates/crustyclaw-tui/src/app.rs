//! Core TUI application state and event handling.

use std::time::Instant;

use crustyclaw_config::AppConfig;
use crustyclaw_core::LogReader;

use crate::keymap::{Action, KeyMapper};
use crate::panels::{ConfigPanel, DashboardPanel, LogsPanel, MessagesPanel, PanelState};

/// The panels available in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Dashboard,
    Logs,
    Messages,
    Config,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Dashboard => "Dashboard",
            Panel::Logs => "Logs",
            Panel::Messages => "Messages",
            Panel::Config => "Config",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Panel::Dashboard => 0,
            Panel::Logs => 1,
            Panel::Messages => 2,
            Panel::Config => 3,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Panel::Dashboard => Panel::Logs,
            Panel::Logs => Panel::Messages,
            Panel::Messages => Panel::Config,
            Panel::Config => Panel::Dashboard,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Panel::Dashboard => Panel::Config,
            Panel::Logs => Panel::Dashboard,
            Panel::Messages => Panel::Logs,
            Panel::Config => Panel::Messages,
        }
    }
}

const ALL_PANELS: [Panel; 4] = [
    Panel::Dashboard,
    Panel::Logs,
    Panel::Messages,
    Panel::Config,
];

/// TUI application state.
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,

    /// Currently selected panel.
    pub active_panel: Panel,

    /// Application start time (for uptime).
    pub start_time: Instant,

    /// Key mapper for vim-style bindings.
    pub keymap: KeyMapper,

    /// Dashboard panel state.
    pub dashboard: DashboardPanel,

    /// Logs panel state.
    pub logs: LogsPanel,

    /// Messages panel state.
    pub messages: MessagesPanel,

    /// Config panel state.
    pub config_panel: ConfigPanel,
}

impl App {
    /// Create a new App with the given configuration and log reader.
    pub fn new(config: AppConfig, log_reader: LogReader) -> Self {
        let config_toml =
            toml::to_string_pretty(&config).unwrap_or_else(|e| format!("(error: {e})"));

        Self {
            should_quit: false,
            active_panel: Panel::Dashboard,
            start_time: Instant::now(),
            keymap: KeyMapper::new(),
            dashboard: DashboardPanel::new(&config),
            logs: LogsPanel::new(log_reader),
            messages: MessagesPanel::new(),
            config_panel: ConfigPanel::new(config_toml),
        }
    }

    /// Process a resolved action.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::NextPanel => self.active_panel = self.active_panel.next(),
            Action::PrevPanel => self.active_panel = self.active_panel.prev(),
            Action::GoToPanel(n) => {
                if let Some(&panel) = ALL_PANELS.get(n) {
                    self.active_panel = panel;
                }
            }
            Action::ScrollDown => self.active_panel_state_mut().scroll_down(1),
            Action::ScrollUp => self.active_panel_state_mut().scroll_up(1),
            Action::HalfPageDown => self.active_panel_state_mut().scroll_down(10),
            Action::HalfPageUp => self.active_panel_state_mut().scroll_up(10),
            Action::ScrollToTop => self.active_panel_state_mut().scroll_to_top(),
            Action::ScrollToBottom => self.active_panel_state_mut().scroll_to_bottom(),
            Action::None => {}
        }
    }

    /// Tick: refresh data from live sources (log reader, etc).
    pub fn tick(&mut self) {
        self.logs.refresh();
        self.dashboard.uptime = self.start_time.elapsed();
    }

    fn active_panel_state_mut(&mut self) -> &mut dyn PanelState {
        match self.active_panel {
            Panel::Dashboard => &mut self.dashboard,
            Panel::Logs => &mut self.logs,
            Panel::Messages => &mut self.messages,
            Panel::Config => &mut self.config_panel,
        }
    }

    /// Get the status line text.
    pub fn status_line(&self) -> String {
        format!(
            " q:quit  Tab/l:next  BackTab/h:prev  j/k:scroll  g/G:top/bottom  1-4:panels  [{panel}]",
            panel = self.active_panel.title()
        )
    }
}
