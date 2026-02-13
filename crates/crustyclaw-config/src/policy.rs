//! Security policy engine — role-based access control.
//!
//! Policies are defined as a set of rules mapping `(role, action, resource)` triples
//! to an [`Effect`] (allow or deny). The [`PolicyEngine`] evaluates these rules
//! in priority order.
//!
//! Policies can be defined programmatically or via the `security_policy!` macro
//! in `crustyclaw-macros`.

use std::collections::HashMap;

/// The effect of a policy rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The action is allowed.
    Allow,
    /// The action is denied.
    Deny,
}

/// A single policy rule.
#[derive(Debug, Clone)]
pub struct PolicyRule {
    /// Role this rule applies to (e.g. "admin", "user", "*").
    pub role: String,
    /// Action being controlled (e.g. "read", "write", "execute", "*").
    pub action: String,
    /// Resource being accessed (e.g. "config", "skills", "messages", "*").
    pub resource: String,
    /// Whether to allow or deny.
    pub effect: Effect,
    /// Priority (higher = evaluated first). Rules with equal priority
    /// are evaluated in insertion order.
    pub priority: u32,
}

impl PolicyRule {
    /// Create a new Allow rule.
    pub fn allow(role: &str, action: &str, resource: &str) -> Self {
        Self {
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            effect: Effect::Allow,
            priority: 0,
        }
    }

    /// Create a new Deny rule.
    pub fn deny(role: &str, action: &str, resource: &str) -> Self {
        Self {
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            effect: Effect::Deny,
            priority: 0,
        }
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check whether this rule matches the given request.
    fn matches(&self, role: &str, action: &str, resource: &str) -> bool {
        (self.role == "*" || self.role == role)
            && (self.action == "*" || self.action == action)
            && (self.resource == "*" || self.resource == resource)
    }
}

/// The result of a policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Allowed by a matching rule.
    Allowed,
    /// Denied by a matching rule.
    Denied,
    /// No matching rule found (default deny).
    NoMatch,
}

/// A compiled policy engine that evaluates access requests.
///
/// Rules are sorted by priority (descending) at evaluation time.
/// The first matching rule wins. If no rule matches, the default
/// is [`PolicyDecision::NoMatch`] (typically treated as deny).
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
    /// Cache of compiled (sorted) rules. Rebuilt when dirty.
    sorted: Vec<PolicyRule>,
    dirty: bool,
}

impl PolicyEngine {
    /// Create a new empty policy engine.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            sorted: Vec::new(),
            dirty: true,
        }
    }

    /// Add a rule to the engine.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
        self.dirty = true;
    }

    /// Evaluate an access request against the policy rules.
    ///
    /// Returns the decision (Allowed, Denied, or NoMatch).
    pub fn evaluate(&mut self, role: &str, action: &str, resource: &str) -> PolicyDecision {
        if self.dirty {
            self.rebuild();
        }

        for rule in &self.sorted {
            if rule.matches(role, action, resource) {
                return match rule.effect {
                    Effect::Allow => PolicyDecision::Allowed,
                    Effect::Deny => PolicyDecision::Denied,
                };
            }
        }

        PolicyDecision::NoMatch
    }

    /// Check whether the given request is allowed (convenience method).
    ///
    /// Returns `true` only if a rule explicitly allows it.
    pub fn is_allowed(&mut self, role: &str, action: &str, resource: &str) -> bool {
        self.evaluate(role, action, resource) == PolicyDecision::Allowed
    }

    /// Return the number of rules in the engine.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Return a summary of all roles referenced by rules.
    pub fn roles(&self) -> Vec<&str> {
        let mut seen = HashMap::new();
        for rule in &self.rules {
            if rule.role != "*" {
                seen.entry(rule.role.as_str()).or_insert(());
            }
        }
        seen.into_keys().collect()
    }

    fn rebuild(&mut self) {
        self.sorted = self.rules.clone();
        // Sort by priority descending (higher priority first)
        self.sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.dirty = false;
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a pre-configured policy engine from a list of rules.
///
/// This is the runtime companion to the `security_policy!` macro.
pub fn build_policy(rules: Vec<PolicyRule>) -> PolicyEngine {
    let mut engine = PolicyEngine::new();
    for rule in rules {
        engine.add_rule(rule);
    }
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_allow() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("admin", "*", "*"));

        assert!(engine.is_allowed("admin", "read", "config"));
        assert!(engine.is_allowed("admin", "write", "skills"));
    }

    #[test]
    fn test_deny_overrides_allow_by_priority() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("user", "*", "*").with_priority(0));
        engine.add_rule(PolicyRule::deny("user", "write", "config").with_priority(10));

        assert!(engine.is_allowed("user", "read", "config"));
        assert!(!engine.is_allowed("user", "write", "config"));
    }

    #[test]
    fn test_no_match_defaults_deny() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("admin", "*", "*"));

        assert_eq!(
            engine.evaluate("unknown", "read", "anything"),
            PolicyDecision::NoMatch
        );
        assert!(!engine.is_allowed("unknown", "read", "anything"));
    }

    #[test]
    fn test_wildcard_role() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("*", "read", "public"));

        assert!(engine.is_allowed("admin", "read", "public"));
        assert!(engine.is_allowed("guest", "read", "public"));
        assert!(!engine.is_allowed("guest", "write", "public"));
    }

    #[test]
    fn test_wildcard_action() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("admin", "*", "config"));

        assert!(engine.is_allowed("admin", "read", "config"));
        assert!(engine.is_allowed("admin", "write", "config"));
        assert!(!engine.is_allowed("admin", "read", "secrets"));
    }

    #[test]
    fn test_build_policy_helper() {
        let mut engine = build_policy(vec![
            PolicyRule::deny("*", "*", "*").with_priority(0),
            PolicyRule::allow("admin", "*", "*").with_priority(10),
        ]);

        assert!(engine.is_allowed("admin", "write", "config"));
        assert!(!engine.is_allowed("user", "write", "config"));
    }

    #[test]
    fn test_roles_listing() {
        let engine = build_policy(vec![
            PolicyRule::allow("admin", "*", "*"),
            PolicyRule::allow("user", "read", "*"),
            PolicyRule::deny("*", "write", "secrets"),
        ]);

        let mut roles = engine.roles();
        roles.sort();
        assert_eq!(roles, vec!["admin", "user"]);
    }

    #[test]
    fn test_rule_count() {
        let engine = build_policy(vec![
            PolicyRule::allow("admin", "*", "*"),
            PolicyRule::deny("user", "write", "config"),
        ]);
        assert_eq!(engine.rule_count(), 2);
    }

    #[test]
    fn test_priority_ordering() {
        let mut engine = PolicyEngine::new();
        // Low priority allow-all
        engine.add_rule(PolicyRule::allow("user", "*", "*").with_priority(1));
        // High priority deny on secrets
        engine.add_rule(PolicyRule::deny("user", "*", "secrets").with_priority(100));
        // Medium priority allow read on secrets
        engine.add_rule(PolicyRule::allow("user", "read", "secrets").with_priority(50));

        // Deny (priority 100) beats allow-read (priority 50)
        assert!(!engine.is_allowed("user", "read", "secrets"));
        // Allow-all (priority 1) works for non-secrets
        assert!(engine.is_allowed("user", "read", "config"));
    }
}
