//! LLM provider integration — multi-provider chat completions with tool use.
//!
//! CrustyClaw supports multiple LLM providers through a unified [`LlmProvider`] trait.
//! Currently supported:
//!
//! - **Anthropic** — Claude models via the Messages API
//! - **OpenAI** — GPT models via the Chat Completions API (also compatible with
//!   Ollama, vLLM, Together AI, and other OpenAI-compatible endpoints)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌──────────────┐
//! │ Skill Engine │────▶│ LlmProvider  │  (trait)
//! └─────────────┘     └──────┬───────┘
//!                            │
//!              ┌─────────────┼─────────────┐
//!              ▼             ▼             ▼
//!     ┌──────────────┐ ┌──────────┐ ┌──────────┐
//!     │  Anthropic   │ │  OpenAI  │ │  Custom  │
//!     │ (Claude API) │ │ (GPT API)│ │ (future) │
//!     └──────────────┘ └──────────┘ └──────────┘
//! ```

pub mod anthropic;
pub mod openai;
pub mod provider;
pub mod types;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;
pub use provider::{LlmError, LlmProvider};
pub use types::*;

/// Create an LLM provider from config.
///
/// Reads the `[llm]` section of the config to determine which provider
/// to use and how to authenticate.
pub fn create_provider(config: &crustyclaw_config::LlmConfig) -> Box<dyn LlmProvider> {
    use crustyclaw_config::LlmProviderKind;

    match config.provider {
        LlmProviderKind::Anthropic => {
            let mut provider = AnthropicProvider::new(&config.api_key);
            if !config.model.is_empty() {
                provider = provider.with_model(&config.model);
            }
            Box::new(provider)
        }
        LlmProviderKind::OpenAi => {
            let mut provider = OpenAiProvider::new(&config.api_key);
            if !config.model.is_empty() {
                provider = provider.with_model(&config.model);
            }
            if let Some(ref base_url) = config.base_url {
                provider = provider.with_base_url(base_url);
            }
            Box::new(provider)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crustyclaw_config::{LlmConfig, LlmProviderKind};

    #[test]
    fn test_create_anthropic_provider() {
        let config = LlmConfig {
            provider: LlmProviderKind::Anthropic,
            api_key: "test-key".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            base_url: None,
            max_tokens: 4096,
            temperature: 0.0,
        };
        let provider = create_provider(&config);
        assert_eq!(provider.name(), "Anthropic");
    }

    #[test]
    fn test_create_openai_provider() {
        let config = LlmConfig {
            provider: LlmProviderKind::OpenAi,
            api_key: "test-key".to_string(),
            model: "gpt-4o".to_string(),
            base_url: None,
            max_tokens: 4096,
            temperature: 0.7,
        };
        let provider = create_provider(&config);
        assert_eq!(provider.name(), "OpenAI");
    }
}
