//! Context window management — token budgets and priority-based context packing.
//!
//! The context window is the total token budget available for an LLM invocation.
//! This module manages how that budget is allocated across:
//!
//! - **System prompt** (fixed, highest priority)
//! - **Tool definitions** (fixed, high priority)
//! - **Conversation history** (dynamic, medium priority)
//! - **Code context** (dynamic, from tree-sitter index, lower priority)
//! - **RAG results** (dynamic, lowest priority)
//!
//! Context items are packed greedily by priority until the budget is exhausted.

use serde::{Deserialize, Serialize};

/// A chunk of context with priority and estimated token count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// What kind of context this is.
    pub kind: ContextKind,
    /// The text content.
    pub content: String,
    /// Estimated token count (rough: ~4 chars per token).
    pub estimated_tokens: u32,
    /// Priority (higher = packed first).
    pub priority: u32,
    /// Source identifier (e.g. file path, "conversation", "system").
    pub source: String,
}

/// The kind of context item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextKind {
    /// System prompt.
    System,
    /// Tool definitions JSON.
    Tools,
    /// Conversation message.
    Conversation,
    /// Code snippet from the codebase.
    Code,
    /// RAG retrieval result.
    Retrieval,
}

/// The context window manager.
///
/// Packs context items into a fixed token budget, prioritizing higher-priority
/// items first. Items that don't fit are dropped.
pub struct ContextWindow {
    /// Total token budget for this invocation.
    budget: u32,
    /// Reserved tokens for the model's response.
    reserved_for_response: u32,
    /// Items packed into the window.
    items: Vec<ContextItem>,
    /// Total tokens used.
    used_tokens: u32,
}

impl ContextWindow {
    /// Create a new context window with the given budget.
    ///
    /// `reserved_for_response` tokens are held back for the model's reply.
    pub fn new(budget: u32, reserved_for_response: u32) -> Self {
        Self {
            budget,
            reserved_for_response,
            items: Vec::new(),
            used_tokens: 0,
        }
    }

    /// Available tokens for context (budget minus response reservation).
    pub fn available(&self) -> u32 {
        self.budget
            .saturating_sub(self.reserved_for_response)
            .saturating_sub(self.used_tokens)
    }

    /// Total tokens used by packed context.
    pub fn used(&self) -> u32 {
        self.used_tokens
    }

    /// Total budget.
    pub fn budget(&self) -> u32 {
        self.budget
    }

    /// Number of packed items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Add a context item if it fits within the budget.
    ///
    /// Returns `true` if the item was added, `false` if it didn't fit.
    pub fn add(&mut self, item: ContextItem) -> bool {
        if item.estimated_tokens <= self.available() {
            self.used_tokens += item.estimated_tokens;
            self.items.push(item);
            true
        } else {
            false
        }
    }

    /// Pack a set of context items by priority.
    ///
    /// Items are sorted by priority (descending) and added greedily.
    /// Returns the number of items that were packed.
    pub fn pack(&mut self, mut items: Vec<ContextItem>) -> usize {
        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        let mut packed = 0;
        for item in items {
            if self.add(item) {
                packed += 1;
            }
        }
        packed
    }

    /// Get the packed items, sorted by kind for consistent prompt assembly.
    pub fn items(&self) -> &[ContextItem] {
        &self.items
    }

    /// Assemble the packed context into ordered sections for the prompt.
    ///
    /// Returns items grouped by kind in the order:
    /// System → Tools → Code → Retrieval → Conversation
    pub fn assemble(&self) -> Vec<&ContextItem> {
        let kind_order = |k: &ContextKind| -> u8 {
            match k {
                ContextKind::System => 0,
                ContextKind::Tools => 1,
                ContextKind::Code => 2,
                ContextKind::Retrieval => 3,
                ContextKind::Conversation => 4,
            }
        };

        let mut ordered: Vec<&ContextItem> = self.items.iter().collect();
        ordered.sort_by_key(|item| kind_order(&item.kind));
        ordered
    }

    /// Estimate the token count for a string (~4 chars per token).
    pub fn estimate_tokens(text: &str) -> u32 {
        // Rough approximation: 1 token ≈ 4 characters
        (text.len() as u32).div_ceil(4)
    }

    /// Create a ContextItem from text with automatic token estimation.
    pub fn item(kind: ContextKind, content: String, priority: u32, source: String) -> ContextItem {
        let estimated_tokens = Self::estimate_tokens(&content);
        ContextItem {
            kind,
            content,
            estimated_tokens,
            priority,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_window() {
        let window = ContextWindow::new(8000, 4000);
        assert_eq!(window.available(), 4000);
        assert_eq!(window.used(), 0);
        assert_eq!(window.item_count(), 0);
    }

    #[test]
    fn test_add_item() {
        let mut window = ContextWindow::new(8000, 4000);
        let item = ContextWindow::item(
            ContextKind::System,
            "You are a helpful assistant.".to_string(),
            100,
            "system".to_string(),
        );
        assert!(window.add(item));
        assert!(window.used() > 0);
        assert_eq!(window.item_count(), 1);
    }

    #[test]
    fn test_budget_exhausted() {
        let mut window = ContextWindow::new(100, 50);
        // Available = 50 tokens ≈ 200 chars
        let big = ContextWindow::item(
            ContextKind::Code,
            "x".repeat(400), // ~100 tokens
            50,
            "file.rs".to_string(),
        );
        assert!(!window.add(big)); // doesn't fit
        assert_eq!(window.item_count(), 0);
    }

    #[test]
    fn test_pack_by_priority() {
        let mut window = ContextWindow::new(1000, 200);
        let items = vec![
            ContextWindow::item(
                ContextKind::Conversation,
                "User message".to_string(),
                50,
                "conversation".to_string(),
            ),
            ContextWindow::item(
                ContextKind::System,
                "System prompt".to_string(),
                100,
                "system".to_string(),
            ),
            ContextWindow::item(
                ContextKind::Code,
                "fn main() {}".to_string(),
                75,
                "main.rs".to_string(),
            ),
        ];
        let packed = window.pack(items);
        assert_eq!(packed, 3); // all fit

        // Check that items were packed (order doesn't matter for add)
        assert_eq!(window.item_count(), 3);
    }

    #[test]
    fn test_assemble_ordering() {
        let mut window = ContextWindow::new(10000, 2000);
        window.add(ContextWindow::item(
            ContextKind::Conversation,
            "Hi!".to_string(),
            50,
            "conv".to_string(),
        ));
        window.add(ContextWindow::item(
            ContextKind::System,
            "You are helpful.".to_string(),
            100,
            "system".to_string(),
        ));
        window.add(ContextWindow::item(
            ContextKind::Code,
            "fn main() {}".to_string(),
            75,
            "main.rs".to_string(),
        ));

        let assembled = window.assemble();
        assert_eq!(assembled[0].kind, ContextKind::System);
        assert_eq!(assembled[1].kind, ContextKind::Code);
        assert_eq!(assembled[2].kind, ContextKind::Conversation);
    }

    #[test]
    fn test_token_estimation() {
        // "hello world" = 11 chars ≈ 3 tokens
        assert_eq!(ContextWindow::estimate_tokens("hello world"), 3);
        // Empty string = 0 tokens
        assert_eq!(ContextWindow::estimate_tokens(""), 0);
        // 100 chars ≈ 25 tokens
        assert_eq!(ContextWindow::estimate_tokens(&"x".repeat(100)), 25);
    }
}
