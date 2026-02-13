//! Skill engine — defines the trait and registry for agent skills.
//!
//! Skills can run directly (in-process) or inside an isolation
//! [`Sandbox`](crate::isolation::Sandbox) for untrusted / third-party code.

use std::collections::HashMap;

use crate::isolation::{self, SandboxConfig};
use crate::message::Envelope;

/// A skill that the agent can execute in response to messages.
pub trait Skill: Send + Sync {
    /// The unique name of this skill.
    fn name(&self) -> &str;

    /// A short description of what this skill does.
    fn description(&self) -> &str;

    /// Whether this skill runs inside an isolation sandbox.
    fn isolated(&self) -> bool {
        false
    }

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

    #[error("sandbox error: {0}")]
    Isolation(#[from] isolation::IsolationError),
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

/// A skill that runs a command inside an isolation sandbox.
///
/// Wraps an external executable (Forgejo Action, script, binary) so that
/// it executes inside a platform-appropriate sandbox. The message body is
/// passed via the `CRUSTYCLAW_MESSAGE` environment variable.
pub struct IsolatedSkill {
    skill_name: String,
    skill_description: String,
    /// The command argv to run inside the sandbox.
    command: Vec<String>,
    /// Sandbox configuration (resource limits, mounts, network policy).
    sandbox_config: SandboxConfig,
    /// The isolation backend to use.
    backend: Box<dyn isolation::SandboxBackend>,
}

impl IsolatedSkill {
    /// Create a new isolated skill.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        command: Vec<String>,
        sandbox_config: SandboxConfig,
        backend: Box<dyn isolation::SandboxBackend>,
    ) -> Self {
        Self {
            skill_name: name.into(),
            skill_description: description.into(),
            command,
            sandbox_config,
            backend,
        }
    }
}

impl Skill for IsolatedSkill {
    fn name(&self) -> &str {
        &self.skill_name
    }

    fn description(&self) -> &str {
        &self.skill_description
    }

    fn isolated(&self) -> bool {
        true
    }

    fn execute(
        &self,
        message: &Envelope,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, SkillError>> + Send + '_>>
    {
        let body = message.body.clone();
        let channel = message.channel.clone();

        Box::pin(async move {
            // Build a per-invocation config with the message injected as env vars
            let config = self
                .sandbox_config
                .clone()
                .with_env("CRUSTYCLAW_MESSAGE", &body)
                .with_env("CRUSTYCLAW_CHANNEL", &channel);

            config.validate()?;

            let result = self.backend.execute(&config, &self.command).await?;

            if result.success() {
                Ok(result.stdout)
            } else {
                Err(SkillError::Execution(format!(
                    "skill '{}' exited with code {}: {}",
                    self.skill_name,
                    result.exit_code,
                    result.stderr.trim()
                )))
            }
        })
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

    #[test]
    fn test_isolated_skill_properties() {
        let config = SandboxConfig::new("test-skill").with_workdir("/tmp");
        let skill = IsolatedSkill::new(
            "echo-skill",
            "Echoes the message",
            vec!["echo".to_string(), "hello".to_string()],
            config,
            Box::new(isolation::NoopBackend),
        );

        assert_eq!(skill.name(), "echo-skill");
        assert_eq!(skill.description(), "Echoes the message");
        assert!(skill.isolated());
    }

    #[tokio::test]
    async fn test_isolated_skill_execute() {
        let config = SandboxConfig::new("echo-test").with_workdir("/tmp");
        let skill = IsolatedSkill::new(
            "echo-skill",
            "Echoes via sandbox",
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo $CRUSTYCLAW_MESSAGE".to_string(),
            ],
            config,
            Box::new(isolation::NoopBackend),
        );

        let envelope = Envelope::new("test", "hello from isolation");
        let result = skill.execute(&envelope).await.unwrap();
        assert_eq!(result.trim(), "hello from isolation");
    }

    #[tokio::test]
    async fn test_isolated_skill_failure() {
        let config = SandboxConfig::new("fail-test").with_workdir("/tmp");
        let skill = IsolatedSkill::new(
            "fail-skill",
            "Always fails",
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo 'error' >&2; exit 1".to_string(),
            ],
            config,
            Box::new(isolation::NoopBackend),
        );

        let envelope = Envelope::new("test", "trigger");
        let result = skill.execute(&envelope).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("fail-skill"));
        assert!(err.contains("error"));
    }

    #[tokio::test]
    async fn test_isolated_skill_in_registry() {
        let config = SandboxConfig::new("reg-test").with_workdir("/tmp");
        let skill = IsolatedSkill::new(
            "sandbox-echo",
            "Sandboxed echo",
            vec!["echo".to_string(), "hi".to_string()],
            config,
            Box::new(isolation::NoopBackend),
        );

        let mut registry = SkillRegistry::new();
        registry.register(Box::new(skill));

        let found = registry.get("sandbox-echo").unwrap();
        assert!(found.isolated());
        assert_eq!(found.description(), "Sandboxed echo");
    }
}
