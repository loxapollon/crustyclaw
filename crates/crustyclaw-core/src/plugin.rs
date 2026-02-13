//! Plugin registry for Forgejo Action extensions.
//!
//! Provides a runtime registry where action plugins register themselves.
//! Plugins discovered at startup are stored here and can be looked up
//! by name for execution.

use std::collections::HashMap;

/// Metadata about a registered action plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name (e.g. "greeting").
    pub name: String,
    /// Plugin version (e.g. "1.0.0").
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// List of input parameter names.
    pub inputs: Vec<String>,
}

/// A hook registration entry.
#[derive(Debug, Clone)]
pub struct HookEntry {
    /// The function/handler name.
    pub handler_name: String,
    /// The event this hook responds to.
    pub event: String,
    /// Priority (higher = runs first).
    pub priority: u32,
}

/// Registry of action plugins and hooks.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginInfo>,
    hooks: Vec<HookEntry>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            hooks: Vec::new(),
        }
    }

    /// Register a plugin.
    pub fn register_plugin(&mut self, info: PluginInfo) {
        self.plugins.insert(info.name.clone(), info);
    }

    /// Register a hook entry.
    pub fn register_hook(&mut self, entry: HookEntry) {
        self.hooks.push(entry);
        // Keep hooks sorted by priority (highest first)
        self.hooks.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Look up a plugin by name.
    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// List all registered plugin names.
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Get all hooks for a given event, sorted by priority (highest first).
    pub fn hooks_for_event(&self, event: &str) -> Vec<&HookEntry> {
        self.hooks.iter().filter(|h| h.event == event).collect()
    }

    /// Get all registered hooks.
    pub fn all_hooks(&self) -> &[HookEntry] {
        &self.hooks
    }

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Number of registered hooks.
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        registry.register_plugin(PluginInfo {
            name: "greeting".to_string(),
            version: "1.0.0".to_string(),
            description: "Says hello".to_string(),
            inputs: vec!["name".to_string(), "greeting".to_string()],
        });

        assert_eq!(registry.plugin_count(), 1);
        let plugin = registry.get_plugin("greeting").unwrap();
        assert_eq!(plugin.version, "1.0.0");
        assert_eq!(plugin.inputs.len(), 2);
    }

    #[test]
    fn test_register_hooks() {
        let mut registry = PluginRegistry::new();
        registry.register_hook(HookEntry {
            handler_name: "handler_a".to_string(),
            event: "on_message".to_string(),
            priority: 5,
        });
        registry.register_hook(HookEntry {
            handler_name: "handler_b".to_string(),
            event: "on_message".to_string(),
            priority: 10,
        });
        registry.register_hook(HookEntry {
            handler_name: "handler_c".to_string(),
            event: "on_startup".to_string(),
            priority: 1,
        });

        assert_eq!(registry.hook_count(), 3);

        let msg_hooks = registry.hooks_for_event("on_message");
        assert_eq!(msg_hooks.len(), 2);
        // Higher priority first
        assert_eq!(msg_hooks[0].handler_name, "handler_b");
        assert_eq!(msg_hooks[1].handler_name, "handler_a");

        let startup_hooks = registry.hooks_for_event("on_startup");
        assert_eq!(startup_hooks.len(), 1);
    }

    #[test]
    fn test_plugin_names() {
        let mut registry = PluginRegistry::new();
        registry.register_plugin(PluginInfo {
            name: "alpha".to_string(),
            version: "1.0".to_string(),
            description: String::new(),
            inputs: Vec::new(),
        });
        registry.register_plugin(PluginInfo {
            name: "beta".to_string(),
            version: "2.0".to_string(),
            description: String::new(),
            inputs: Vec::new(),
        });

        let mut names = registry.plugin_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_empty_registry() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.plugin_count(), 0);
        assert_eq!(registry.hook_count(), 0);
        assert!(registry.hooks_for_event("any").is_empty());
        assert!(registry.get_plugin("none").is_none());
    }
}
