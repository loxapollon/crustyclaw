//! Tool registry — MCP-compatible tool definitions with per-task scoping.
//!
//! Tools are the actions an LLM can invoke during skill execution.
//! Each tool has a name, description, JSON Schema for parameters, and
//! a trust level that determines which isolation contexts can use it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::llm::types::ToolDefinition;

/// Trust level required to invoke a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ToolTrust {
    /// Available to all contexts including LLM-generated code.
    Public,
    /// Available to internal and trusted contexts.
    #[default]
    Internal,
    /// Only available to explicitly trusted contexts.
    Trusted,
    /// Only available to the daemon itself (system tools).
    System,
}

/// A registered tool with metadata.
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    /// Tool definition (name, description, parameters schema).
    pub definition: ToolDefinition,
    /// Minimum trust level required to invoke this tool.
    pub trust: ToolTrust,
    /// Tags for scoping (e.g. "code", "search", "system", "file").
    pub tags: Vec<String>,
    /// Whether this tool is enabled.
    pub enabled: bool,
}

/// Registry of all available tools.
///
/// Tools can be filtered by trust level and tags to create per-task
/// scoped tool sets.
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

impl ToolRegistry {
    /// Create a new empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with the built-in default tools.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: RegisteredTool) {
        self.tools.insert(tool.definition.name.clone(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.get(name)
    }

    /// List all registered tool names.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.tools.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all tool definitions (for sending to the LLM).
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.enabled)
            .map(|t| t.definition.clone())
            .collect()
    }

    /// Get tool definitions filtered by trust level and optional tags.
    ///
    /// Returns only tools where the caller's trust level meets or exceeds
    /// the tool's required trust.
    pub fn scoped_definitions(
        &self,
        caller_trust: ToolTrust,
        tags: Option<&[&str]>,
    ) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.enabled && t.trust <= caller_trust)
            .filter(|t| {
                if let Some(required_tags) = tags {
                    required_tags
                        .iter()
                        .any(|tag| t.tags.iter().any(|tt| tt == tag))
                } else {
                    true
                }
            })
            .map(|t| t.definition.clone())
            .collect()
    }

    /// Register the built-in tools.
    fn register_defaults(&mut self) {
        // Code search tools
        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "search_code".to_string(),
                description: "Search the codebase for a pattern using regex or literal string."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Search pattern (regex supported)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Optional directory to scope the search"
                        },
                        "file_type": {
                            "type": "string",
                            "description": "File extension filter (e.g. 'rs', 'ts', 'py')"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
            trust: ToolTrust::Public,
            tags: vec!["code".to_string(), "search".to_string()],
            enabled: true,
        });

        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "read_file".to_string(),
                description: "Read the contents of a file.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        },
                        "start_line": {
                            "type": "integer",
                            "description": "Starting line number (1-indexed)"
                        },
                        "end_line": {
                            "type": "integer",
                            "description": "Ending line number (inclusive)"
                        }
                    },
                    "required": ["path"]
                }),
            },
            trust: ToolTrust::Public,
            tags: vec!["code".to_string(), "file".to_string()],
            enabled: true,
        });

        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "list_files".to_string(),
                description: "List files matching a glob pattern.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Glob pattern (e.g. 'src/**/*.rs')"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
            trust: ToolTrust::Public,
            tags: vec!["code".to_string(), "file".to_string()],
            enabled: true,
        });

        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "list_symbols".to_string(),
                description: "List code symbols (functions, structs, types) in a file or directory."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File or directory path"
                        },
                        "kind": {
                            "type": "string",
                            "description": "Symbol kind filter: 'function', 'struct', 'type', 'impl', 'all'",
                            "enum": ["function", "struct", "type", "impl", "all"]
                        }
                    },
                    "required": ["path"]
                }),
            },
            trust: ToolTrust::Public,
            tags: vec!["code".to_string(), "search".to_string()],
            enabled: true,
        });

        // Execution tools
        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "run_command".to_string(),
                description: "Execute a shell command in a sandboxed environment.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        },
                        "working_dir": {
                            "type": "string",
                            "description": "Working directory for the command"
                        },
                        "timeout_secs": {
                            "type": "integer",
                            "description": "Timeout in seconds (default 60)"
                        }
                    },
                    "required": ["command"]
                }),
            },
            trust: ToolTrust::Internal,
            tags: vec!["execution".to_string()],
            enabled: true,
        });

        // System tools (daemon management)
        self.register(RegisteredTool {
            definition: ToolDefinition {
                name: "daemon_status".to_string(),
                description: "Get the current daemon status and configuration.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            trust: ToolTrust::System,
            tags: vec!["system".to_string()],
            enabled: true,
        });
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_registry() {
        let reg = ToolRegistry::new();
        assert!(reg.names().is_empty());
        assert!(reg.definitions().is_empty());
    }

    #[test]
    fn test_defaults_registered() {
        let reg = ToolRegistry::with_defaults();
        let names = reg.names();
        assert!(names.contains(&"search_code".to_string()));
        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&"list_files".to_string()));
        assert!(names.contains(&"list_symbols".to_string()));
        assert!(names.contains(&"run_command".to_string()));
        assert!(names.contains(&"daemon_status".to_string()));
    }

    #[test]
    fn test_scoped_definitions_public() {
        let reg = ToolRegistry::with_defaults();
        let public = reg.scoped_definitions(ToolTrust::Public, None);
        // Public callers should only see Public tools
        for def in &public {
            let tool = reg.get(&def.name).unwrap();
            assert_eq!(tool.trust, ToolTrust::Public);
        }
    }

    #[test]
    fn test_scoped_definitions_internal() {
        let reg = ToolRegistry::with_defaults();
        let internal = reg.scoped_definitions(ToolTrust::Internal, None);
        // Internal callers see Public + Internal tools
        for def in &internal {
            let tool = reg.get(&def.name).unwrap();
            assert!(tool.trust <= ToolTrust::Internal);
        }
        // Should include run_command (Internal)
        assert!(internal.iter().any(|d| d.name == "run_command"));
    }

    #[test]
    fn test_scoped_definitions_system() {
        let reg = ToolRegistry::with_defaults();
        let system = reg.scoped_definitions(ToolTrust::System, None);
        // System callers see everything
        assert!(system.iter().any(|d| d.name == "daemon_status"));
        assert!(system.iter().any(|d| d.name == "search_code"));
        assert!(system.iter().any(|d| d.name == "run_command"));
    }

    #[test]
    fn test_scoped_by_tags() {
        let reg = ToolRegistry::with_defaults();
        let code_tools = reg.scoped_definitions(ToolTrust::System, Some(&["code"]));
        assert!(code_tools.iter().any(|d| d.name == "search_code"));
        assert!(code_tools.iter().any(|d| d.name == "read_file"));
        // run_command is not tagged "code"
        assert!(!code_tools.iter().any(|d| d.name == "run_command"));
    }

    #[test]
    fn test_get_tool() {
        let reg = ToolRegistry::with_defaults();
        let tool = reg.get("search_code").unwrap();
        assert_eq!(tool.trust, ToolTrust::Public);
        assert!(tool.tags.contains(&"code".to_string()));
    }

    #[test]
    fn test_disabled_tool_excluded() {
        let mut reg = ToolRegistry::with_defaults();
        if let Some(tool) = reg.tools.get_mut("search_code") {
            tool.enabled = false;
        }
        let defs = reg.definitions();
        assert!(!defs.iter().any(|d| d.name == "search_code"));
    }
}
