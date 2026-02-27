//! Common types for LLM provider integration.
//!
//! These types define the shared vocabulary for chat completions,
//! tool definitions, and streaming across all LLM providers.

use serde::{Deserialize, Serialize};

/// A chat message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: "system", "user", "assistant", or "tool".
    pub role: String,
    /// Text content of the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool call results (when role = "tool").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls requested by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: Some(content.into()),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// A tool that the model can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g. "search_code", "run_command").
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub parameters: serde_json::Value,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call (for matching results).
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// JSON arguments for the tool.
    pub arguments: serde_json::Value,
}

/// Request for a chat completion.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// Model identifier (e.g. "claude-sonnet-4-20250514", "gpt-4o").
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<ChatMessage>,
    /// Available tools the model may call.
    pub tools: Vec<ToolDefinition>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature (0.0–2.0).
    pub temperature: f32,
    /// Optional system prompt (overrides system message in messages).
    pub system: Option<String>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: 4096,
            temperature: 0.0,
            system: None,
        }
    }
}

/// Response from a chat completion.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// The assistant's response message.
    pub message: ChatMessage,
    /// Finish reason: "stop", "tool_use", "length", etc.
    pub finish_reason: String,
    /// Token usage statistics.
    pub usage: TokenUsage,
    /// Raw model identifier used.
    pub model: String,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A streaming chunk from the model.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A text delta.
    Text(String),
    /// A tool call is being built.
    ToolCallStart { id: String, name: String },
    /// Arguments delta for an in-progress tool call.
    ToolCallDelta { id: String, arguments_delta: String },
    /// The stream has finished.
    Done {
        finish_reason: String,
        usage: Option<TokenUsage>,
    },
}

// Provider kind is defined in crustyclaw_config::LlmProviderKind
