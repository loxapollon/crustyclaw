//! TUI panel implementations.

mod config;
mod dashboard;
mod logs;
mod messages;

pub use config::ConfigPanel;
pub use dashboard::DashboardPanel;
pub use logs::LogsPanel;
pub use messages::MessagesPanel;

/// Trait for panels that support scrolling.
pub trait PanelState {
    /// Scroll down by `n` lines.
    fn scroll_down(&mut self, n: usize);

    /// Scroll up by `n` lines.
    fn scroll_up(&mut self, n: usize);

    /// Scroll to the very top.
    fn scroll_to_top(&mut self);

    /// Scroll to the very bottom.
    fn scroll_to_bottom(&mut self);
}
