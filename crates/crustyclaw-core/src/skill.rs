//! Skill engine — defines the trait and registry for agent skills.

use std::collections::HashMap;

use crate::message::Envelope;

/// A skill that the agent can execute in response to messages.
pub trait Skill: Send + Sync {
    /// The unique name of this skill.
    fn name(&self) -> &str;

    /// A short description of what this skill does.
    fn description(&self) -> &str;

    /// Execute the skill with the given message, returning a response body.
    fn execute(
        &self,
        message: &Envelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, SkillError>> + Send + '_>>;
}

/// Errors from skill execution.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("skill execution failed: {0}")]
    Execution(String),

    #[error("skill not found: {0}")]
    NotFound(String),
}

/// Registry of available skills.
pub struct SkillRegistry {
    skills: HashMap<String, Box<dyn Skill>>,
}

impl SkillRegistry {
    /// Create an empty skill registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill.
    pub fn register(&mut self, skill: Box<dyn Skill>) {
        let name = skill.name().to_string();
        self.skills.insert(name, skill);
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<&dyn Skill> {
        self.skills.get(name).map(|s| s.as_ref())
    }

    /// List all registered skill names.
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let registry = SkillRegistry::new();
        assert!(registry.list().is_empty());
        assert!(registry.get("nonexistent").is_none());
    }
}
