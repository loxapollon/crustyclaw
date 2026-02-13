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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Action;

    fn make_app() -> App {
        let config = AppConfig::default();
        let collector = crustyclaw_core::LogCollector::new(100);
        let reader = collector.reader();
        App::new(config, reader)
    }

    // ── Panel enum tests ──────────────────────────────────────────

    #[test]
    fn test_panel_titles() {
        assert_eq!(Panel::Dashboard.title(), "Dashboard");
        assert_eq!(Panel::Logs.title(), "Logs");
        assert_eq!(Panel::Messages.title(), "Messages");
        assert_eq!(Panel::Config.title(), "Config");
    }

    #[test]
    fn test_panel_indices() {
        assert_eq!(Panel::Dashboard.index(), 0);
        assert_eq!(Panel::Logs.index(), 1);
        assert_eq!(Panel::Messages.index(), 2);
        assert_eq!(Panel::Config.index(), 3);
    }

    #[test]
    fn test_panel_next_wraps() {
        assert_eq!(Panel::Dashboard.next(), Panel::Logs);
        assert_eq!(Panel::Logs.next(), Panel::Messages);
        assert_eq!(Panel::Messages.next(), Panel::Config);
        assert_eq!(Panel::Config.next(), Panel::Dashboard);
    }

    #[test]
    fn test_panel_prev_wraps() {
        assert_eq!(Panel::Dashboard.prev(), Panel::Config);
        assert_eq!(Panel::Config.prev(), Panel::Messages);
        assert_eq!(Panel::Messages.prev(), Panel::Logs);
        assert_eq!(Panel::Logs.prev(), Panel::Dashboard);
    }

    // ── App creation ──────────────────────────────────────────────

    #[test]
    fn test_app_defaults() {
        let app = make_app();
        assert!(!app.should_quit);
        assert_eq!(app.active_panel, Panel::Dashboard);
    }

    // ── Action handling ───────────────────────────────────────────

    #[test]
    fn test_quit_action() {
        let mut app = make_app();
        app.handle_action(Action::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn test_next_prev_panel() {
        let mut app = make_app();
        assert_eq!(app.active_panel, Panel::Dashboard);

        app.handle_action(Action::NextPanel);
        assert_eq!(app.active_panel, Panel::Logs);

        app.handle_action(Action::PrevPanel);
        assert_eq!(app.active_panel, Panel::Dashboard);
    }

    #[test]
    fn test_goto_panel() {
        let mut app = make_app();

        app.handle_action(Action::GoToPanel(2));
        assert_eq!(app.active_panel, Panel::Messages);

        app.handle_action(Action::GoToPanel(0));
        assert_eq!(app.active_panel, Panel::Dashboard);

        // Out of bounds — no change
        app.handle_action(Action::GoToPanel(99));
        assert_eq!(app.active_panel, Panel::Dashboard);
    }

    #[test]
    fn test_full_panel_cycle() {
        let mut app = make_app();
        for _ in 0..4 {
            app.handle_action(Action::NextPanel);
        }
        // Should wrap back to Dashboard
        assert_eq!(app.active_panel, Panel::Dashboard);
    }

    #[test]
    fn test_scroll_actions_no_panic() {
        let mut app = make_app();
        // Scroll on each panel to exercise all PanelState impls
        for i in 0..4 {
            app.handle_action(Action::GoToPanel(i));
            app.handle_action(Action::ScrollDown);
            app.handle_action(Action::ScrollUp);
            app.handle_action(Action::HalfPageDown);
            app.handle_action(Action::HalfPageUp);
            app.handle_action(Action::ScrollToTop);
            app.handle_action(Action::ScrollToBottom);
        }
    }

    #[test]
    fn test_none_action_is_noop() {
        let mut app = make_app();
        let panel_before = app.active_panel;
        app.handle_action(Action::None);
        assert_eq!(app.active_panel, panel_before);
        assert!(!app.should_quit);
    }

    // ── Tick ──────────────────────────────────────────────────────

    #[test]
    fn test_tick_updates_uptime() {
        let mut app = make_app();
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.tick();
        assert!(app.dashboard.uptime.as_millis() > 0);
    }

    // ── Status line ───────────────────────────────────────────────

    #[test]
    fn test_status_line_contains_panel_name() {
        let mut app = make_app();
        assert!(app.status_line().contains("[Dashboard]"));

        app.handle_action(Action::NextPanel);
        assert!(app.status_line().contains("[Logs]"));
    }
}
