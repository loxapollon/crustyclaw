//! OpenAI-compatible API provider.
//!
//! Implements the [`LlmProvider`] trait for OpenAI's Chat Completions API.
//! Also compatible with any provider that follows the OpenAI API format
//! (e.g. Ollama, vLLM, Together AI).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::BoxFuture;

use super::provider::{LlmError, LlmProvider};
use super::types::*;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI-compatible provider.
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: OPENAI_API_URL.to_string(),
            default_model: "gpt-4o".to_string(),
        }
    }

    /// Set the default model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set a custom base URL (for OpenAI-compatible providers).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Convert our ChatRequest into OpenAI's API format.
    fn build_request_body(&self, request: &ChatRequest) -> OpenAiRequest {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let mut messages: Vec<OpenAiMessage> = Vec::new();

        // Add system prompt if provided
        if let Some(ref system) = request.system {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(system.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Convert messages
        for msg in &request.messages {
            messages.push(OpenAiMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                tool_calls: msg.tool_calls.as_ref().map(|calls| {
                    calls
                        .iter()
                        .map(|tc| OpenAiToolCall {
                            id: tc.id.clone(),
                            r#type: "function".to_string(),
                            function: OpenAiFunction {
                                name: tc.name.clone(),
                                arguments: tc.arguments.to_string(),
                            },
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id.clone(),
            });
        }

        // Convert tools
        let tools: Vec<OpenAiTool> = request
            .tools
            .iter()
            .map(|t| OpenAiTool {
                r#type: "function".to_string(),
                function: OpenAiToolFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            })
            .collect();

        OpenAiRequest {
            model,
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            max_tokens: Some(request.max_tokens),
            temperature: Some(request.temperature),
        }
    }

    /// Parse OpenAI's response into our ChatResponse.
    fn parse_response(&self, resp: OpenAiResponse) -> Result<ChatResponse, LlmError> {
        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::Parse("no choices in response".to_string()))?;

        let tool_calls = choice.message.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null),
                })
                .collect()
        });

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => "stop".to_string(),
            Some("tool_calls") => "tool_use".to_string(),
            Some("length") => "length".to_string(),
            Some(other) => other.to_string(),
            None => "unknown".to_string(),
        };

        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: choice.message.content,
                tool_call_id: None,
                tool_calls,
            },
            finish_reason,
            usage: resp.usage.map_or_else(TokenUsage::default, |u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            model: resp.model,
        })
    }
}

impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn chat(&self, request: &ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>> {
        let body = self.build_request_body(request);
        Box::pin(async move {
            debug!(model = %body.model, "OpenAI chat request");

            let resp = self
                .client
                .post(&self.base_url)
                .header("authorization", format!("Bearer {}", self.api_key))
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

            let api_resp: OpenAiResponse = resp
                .json()
                .await
                .map_err(|e| LlmError::Parse(e.to_string()))?;

            self.parse_response(api_resp)
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

// ── OpenAI API types (private) ──────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    r#type: String,
    function: OpenAiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OpenAiTool {
    r#type: String,
    function: OpenAiToolFunction,
}

#[derive(Debug, Serialize)]
struct OpenAiToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_simple_request() {
        let provider = OpenAiProvider::new("test-key");
        let request = ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ChatMessage::user("Hello!")],
            system: Some("You are helpful.".to_string()),
            max_tokens: 1024,
            ..Default::default()
        };

        let body = provider.build_request_body(&request);
        assert_eq!(body.model, "gpt-4o");
        assert_eq!(body.messages.len(), 2); // system + user
        assert_eq!(body.messages[0].role, "system");
        assert_eq!(body.messages[1].role, "user");
    }

    #[test]
    fn test_parse_text_response() {
        let provider = OpenAiProvider::new("test-key");
        let api_resp = OpenAiResponse {
            model: "gpt-4o".to_string(),
            choices: vec![OpenAiChoice {
                message: OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: 5,
                completion_tokens: 3,
                total_tokens: 8,
            }),
        };

        let resp = provider.parse_response(api_resp).unwrap();
        assert_eq!(resp.message.content.as_deref(), Some("Hello!"));
        assert_eq!(resp.finish_reason, "stop");
        assert_eq!(resp.usage.total_tokens, 8);
    }

    #[test]
    fn test_custom_base_url() {
        let provider =
            OpenAiProvider::new("key").with_base_url("http://localhost:11434/v1/chat/completions");
        assert_eq!(
            provider.base_url,
            "http://localhost:11434/v1/chat/completions"
        );
    }
}
