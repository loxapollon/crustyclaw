//! Anthropic Claude API provider.
//!
//! Implements the [`LlmProvider`] trait for the Anthropic Messages API.
//! Supports chat completions with tool use via the `/v1/messages` endpoint.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::BoxFuture;

use super::provider::{LlmError, LlmProvider};
use super::types::*;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic Claude provider.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            default_model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    /// Set the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Convert our ChatRequest into Anthropic's API format.
    fn build_request_body(&self, request: &ChatRequest) -> AnthropicRequest {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        // Extract system prompt
        let system = request.system.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == "system")
                .and_then(|m| m.content.clone())
        });

        // Convert messages (skip system messages, they go in the system field)
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                if m.role == "tool" {
                    AnthropicMessage {
                        role: "user".to_string(),
                        content: AnthropicContent::Blocks(vec![AnthropicBlock::ToolResult {
                            tool_use_id: m.tool_call_id.clone().unwrap_or_default(),
                            content: m.content.clone().unwrap_or_default(),
                        }]),
                    }
                } else if let Some(ref calls) = m.tool_calls {
                    let blocks: Vec<AnthropicBlock> = calls
                        .iter()
                        .map(|tc| AnthropicBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        })
                        .collect();
                    AnthropicMessage {
                        role: m.role.clone(),
                        content: AnthropicContent::Blocks(blocks),
                    }
                } else {
                    AnthropicMessage {
                        role: m.role.clone(),
                        content: AnthropicContent::Text(m.content.clone().unwrap_or_default()),
                    }
                }
            })
            .collect();

        // Convert tools
        let tools: Vec<AnthropicTool> = request
            .tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect();

        AnthropicRequest {
            model,
            max_tokens: request.max_tokens,
            system,
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            temperature: Some(request.temperature),
        }
    }

    /// Parse Anthropic's response into our ChatResponse.
    fn parse_response(&self, resp: AnthropicResponse) -> ChatResponse {
        let mut content = None;
        let mut tool_calls = Vec::new();

        for block in &resp.content {
            match block {
                AnthropicBlock::Text { text } => {
                    content = Some(text.clone());
                }
                AnthropicBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: input.clone(),
                    });
                }
                AnthropicBlock::ToolResult { .. } => {}
            }
        }

        let finish_reason = match resp.stop_reason.as_deref() {
            Some("end_turn") => "stop".to_string(),
            Some("tool_use") => "tool_use".to_string(),
            Some("max_tokens") => "length".to_string(),
            Some(other) => other.to_string(),
            None => "unknown".to_string(),
        };

        ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
                tool_call_id: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
            },
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
            model: resp.model,
        }
    }
}

impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn chat(&self, request: &ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>> {
        let body = self.build_request_body(request);
        Box::pin(async move {
            debug!(model = %body.model, "Anthropic chat request");

            let resp = self
                .client
                .post(ANTHROPIC_API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_API_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| LlmError::Network(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 401 {
                return Err(LlmError::Auth("invalid API key".to_string()));
            }
            if status == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                return Err(LlmError::RateLimited {
                    retry_after_secs: retry_after,
                });
            }
            if !resp.status().is_success() {
                let error_body = resp.text().await.unwrap_or_default();
                return Err(LlmError::ProviderError {
                    status,
                    message: error_body,
                });
            }

            let api_resp: AnthropicResponse = resp
                .json()
                .await
                .map_err(|e| LlmError::Parse(e.to_string()))?;

            Ok(self.parse_response(api_resp))
        })
    }

    fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> BoxFuture<'_, Result<tokio::sync::mpsc::Receiver<Result<StreamChunk, LlmError>>, LlmError>>
    {
        let request = request.clone();
        Box::pin(async move {
            let (tx, rx) = tokio::sync::mpsc::channel(64);

            // For now, fall back to non-streaming and emit as a single chunk
            let response = self.chat(&request).await?;
            tokio::spawn(async move {
                if let Some(ref text) = response.message.content {
                    let _ = tx.send(Ok(StreamChunk::Text(text.clone()))).await;
                }
                if let Some(ref calls) = response.message.tool_calls {
                    for call in calls {
                        let _ = tx
                            .send(Ok(StreamChunk::ToolCallStart {
                                id: call.id.clone(),
                                name: call.name.clone(),
                            }))
                            .await;
                        let _ = tx
                            .send(Ok(StreamChunk::ToolCallDelta {
                                id: call.id.clone(),
                                arguments_delta: call.arguments.to_string(),
                            }))
                            .await;
                    }
                }
                let _ = tx
                    .send(Ok(StreamChunk::Done {
                        finish_reason: response.finish_reason,
                        usage: Some(response.usage),
                    }))
                    .await;
            });

            Ok(rx)
        })
    }
}

// ── Anthropic API types (private) ───────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicBlock>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<AnthropicBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_request() {
        let provider = AnthropicProvider::new("test-key");
        let request = ChatRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![
                ChatMessage::system("You are a helpful assistant."),
                ChatMessage::user("Hello!"),
            ],
            max_tokens: 1024,
            temperature: 0.7,
            ..Default::default()
        };

        let body = provider.build_request_body(&request);
        assert_eq!(body.model, "claude-sonnet-4-20250514");
        assert_eq!(body.max_tokens, 1024);
        assert_eq!(body.system.as_deref(), Some("You are a helpful assistant."));
        // System message is extracted, so only user message remains
        assert_eq!(body.messages.len(), 1);
        assert_eq!(body.messages[0].role, "user");
    }

    #[test]
    fn test_build_request_with_tools() {
        let provider = AnthropicProvider::new("test-key");
        let request = ChatRequest {
            model: "claude-sonnet-4-20250514".to_string(),
            messages: vec![ChatMessage::user("Search for X")],
            tools: vec![ToolDefinition {
                name: "search".to_string(),
                description: "Search the codebase".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
            }],
            max_tokens: 2048,
            ..Default::default()
        };

        let body = provider.build_request_body(&request);
        assert!(body.tools.is_some());
        assert_eq!(body.tools.as_ref().unwrap().len(), 1);
        assert_eq!(body.tools.as_ref().unwrap()[0].name, "search");
    }

    #[test]
    fn test_parse_text_response() {
        let provider = AnthropicProvider::new("test-key");
        let api_resp = AnthropicResponse {
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![AnthropicBlock::Text {
                text: "Hello! How can I help?".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: AnthropicUsage {
                input_tokens: 10,
                output_tokens: 8,
            },
        };

        let resp = provider.parse_response(api_resp);
        assert_eq!(
            resp.message.content.as_deref(),
            Some("Hello! How can I help?")
        );
        assert_eq!(resp.finish_reason, "stop");
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 8);
        assert_eq!(resp.usage.total_tokens, 18);
    }

    #[test]
    fn test_parse_tool_use_response() {
        let provider = AnthropicProvider::new("test-key");
        let api_resp = AnthropicResponse {
            model: "claude-sonnet-4-20250514".to_string(),
            content: vec![AnthropicBlock::ToolUse {
                id: "call_123".to_string(),
                name: "search".to_string(),
                input: serde_json::json!({"query": "hello"}),
            }],
            stop_reason: Some("tool_use".to_string()),
            usage: AnthropicUsage {
                input_tokens: 20,
                output_tokens: 15,
            },
        };

        let resp = provider.parse_response(api_resp);
        assert_eq!(resp.finish_reason, "tool_use");
        let calls = resp.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search");
        assert_eq!(calls[0].id, "call_123");
    }

    #[test]
    fn test_default_model() {
        let provider = AnthropicProvider::new("test-key");
        let request = ChatRequest {
            messages: vec![ChatMessage::user("hi")],
            ..Default::default()
        };
        let body = provider.build_request_body(&request);
        assert_eq!(body.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_custom_model() {
        let provider = AnthropicProvider::new("test-key").with_model("claude-opus-4-20250514");
        let request = ChatRequest {
            messages: vec![ChatMessage::user("hi")],
            ..Default::default()
        };
        let body = provider.build_request_body(&request);
        assert_eq!(body.model, "claude-opus-4-20250514");
    }
}
