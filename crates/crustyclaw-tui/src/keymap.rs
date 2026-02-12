//! Vim-style keybinding system.
//!
//! Maps key events to actions. Supports single keys and simple two-key
//! sequences (e.g. `gg` for scroll-to-top).

use crossterm::event::KeyCode;

/// An action the TUI can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    NextPanel,
    PrevPanel,
    GoToPanel(usize),
    ScrollDown,
    ScrollUp,
    HalfPageDown,
    HalfPageUp,
    ScrollToTop,
    ScrollToBottom,
    None,
}

/// Key mapper with support for multi-key sequences.
pub struct KeyMapper {
    /// Pending first key of a two-key sequence (e.g. the first `g` in `gg`).
    pending: Option<KeyCode>,
}

impl KeyMapper {
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Feed a key code and return the resolved action.
    ///
    /// If the key starts a multi-key sequence, returns `Action::None` and
    /// waits for the next key. If the sequence is invalid, the pending key
    /// is discarded.
    pub fn resolve(&mut self, key: KeyCode) -> Action {
        // Check if we have a pending key from a previous press.
        if let Some(prev) = self.pending.take() {
            return self.resolve_sequence(prev, key);
        }

        match key {
            KeyCode::Char('q') => Action::Quit,

            // Panel switching
            KeyCode::Tab | KeyCode::Char('l') => Action::NextPanel,
            KeyCode::BackTab | KeyCode::Char('h') => Action::PrevPanel,
            KeyCode::Char('1') => Action::GoToPanel(0),
            KeyCode::Char('2') => Action::GoToPanel(1),
            KeyCode::Char('3') => Action::GoToPanel(2),
            KeyCode::Char('4') => Action::GoToPanel(3),

            // Vim scrolling
            KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
            KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
            KeyCode::Char('d') => Action::HalfPageDown,
            KeyCode::Char('u') => Action::HalfPageUp,
            KeyCode::Char('G') => Action::ScrollToBottom,

            // Start of multi-key sequence
            KeyCode::Char('g') => {
                self.pending = Some(key);
                Action::None
            }

            _ => Action::None,
        }
    }

    fn resolve_sequence(&mut self, first: KeyCode, second: KeyCode) -> Action {
        match (first, second) {
            (KeyCode::Char('g'), KeyCode::Char('g')) => Action::ScrollToTop,
            // Unknown sequence — try to interpret the second key as a fresh keypress
            _ => self.resolve(second),
        }
    }
}

impl Default for KeyMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_keys() {
        let mut km = KeyMapper::new();
        assert_eq!(km.resolve(KeyCode::Char('q')), Action::Quit);
        assert_eq!(km.resolve(KeyCode::Char('j')), Action::ScrollDown);
        assert_eq!(km.resolve(KeyCode::Char('k')), Action::ScrollUp);
        assert_eq!(km.resolve(KeyCode::Char('G')), Action::ScrollToBottom);
        assert_eq!(km.resolve(KeyCode::Tab), Action::NextPanel);
        assert_eq!(km.resolve(KeyCode::Char('l')), Action::NextPanel);
        assert_eq!(km.resolve(KeyCode::Char('h')), Action::PrevPanel);
    }

    #[test]
    fn test_gg_sequence() {
        let mut km = KeyMapper::new();
        assert_eq!(km.resolve(KeyCode::Char('g')), Action::None);
        assert_eq!(km.resolve(KeyCode::Char('g')), Action::ScrollToTop);
    }

    #[test]
    fn test_invalid_sequence_falls_through() {
        let mut km = KeyMapper::new();
        // g followed by j should drop g and interpret j
        assert_eq!(km.resolve(KeyCode::Char('g')), Action::None);
        assert_eq!(km.resolve(KeyCode::Char('j')), Action::ScrollDown);
    }

    #[test]
    fn test_number_panels() {
        let mut km = KeyMapper::new();
        assert_eq!(km.resolve(KeyCode::Char('1')), Action::GoToPanel(0));
        assert_eq!(km.resolve(KeyCode::Char('2')), Action::GoToPanel(1));
        assert_eq!(km.resolve(KeyCode::Char('3')), Action::GoToPanel(2));
        assert_eq!(km.resolve(KeyCode::Char('4')), Action::GoToPanel(3));
    }

    #[test]
    fn test_half_page() {
        let mut km = KeyMapper::new();
        assert_eq!(km.resolve(KeyCode::Char('d')), Action::HalfPageDown);
        assert_eq!(km.resolve(KeyCode::Char('u')), Action::HalfPageUp);
    }
}
