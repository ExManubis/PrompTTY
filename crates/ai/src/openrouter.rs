//! Minimal OpenRouter client for single-shot chat completions.
//!
//! Backs the code-review AI features (commit messages, PR titles/bodies) with
//! a user-supplied OpenRouter key, so those flows work without Warp's cloud
//! inference. OpenAI chat-completions schema, non-streaming; the responses
//! are short, so streaming buys nothing.

use std::time::Duration;

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use http_client::Client;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::code_review::CodeReviewAi;
use crate::code_review::api::{
    GenerateCodeReviewContentRequest, GenerateCodeReviewContentResponse,
};
use crate::code_review::prompts::build_messages;

const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_COMPLETION_TOKENS: u32 = 1024;

/// Low temperature: commit messages and PR copy should be near-deterministic.
const COMPLETION_TEMPERATURE: f32 = 0.2;

pub struct OpenRouterClient {
    http_client: Client,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            http_client: Client::new(),
            api_key,
            model,
        }
    }

    /// Runs one non-streaming chat completion and returns the message content.
    /// Errors carry the provider's message so the caller can surface why a
    /// key was rejected or a model is unavailable.
    pub async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let body = completion_request_body(&self.model, system, user);
        let response = self
            .http_client
            .post(format!("{OPENROUTER_API_BASE}/chat/completions"))
            .bearer_auth(&self.api_key)
            .header("X-Title", "PrompTTY")
            .timeout(COMPLETION_TIMEOUT)
            .json(&body)
            .send()
            .await
            .context("OpenRouter request failed")?;

        let response = response.error_for_status_with_body().await.map_err(|err| {
            anyhow!(
                "OpenRouter returned {:?}: {}",
                err.source.status(),
                describe_error_body(err.body.as_deref())
            )
        })?;

        let text = response
            .text()
            .await
            .context("reading OpenRouter response")?;
        parse_completion_content(&text)
    }
}

#[async_trait]
impl CodeReviewAi for OpenRouterClient {
    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error> {
        let (system, user) = build_messages(&request);
        let content = self.complete(&system, &user).await?;
        Ok(GenerateCodeReviewContentResponse { content })
    }
}

/// Builds the chat-completions request body. Split out for unit tests.
fn completion_request_body(model: &str, system: &str, user: &str) -> Value {
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": COMPLETION_TEMPERATURE,
        "max_tokens": MAX_COMPLETION_TOKENS,
        "stream": false,
    })
}

/// Extracts the message content from a chat-completions response body.
fn parse_completion_content(body: &str) -> anyhow::Result<String> {
    let parsed: ChatCompletionResponse =
        serde_json::from_str(body).context("unexpected OpenRouter response shape")?;
    let content = parsed
        .choices
        .first()
        .ok_or_else(|| anyhow!("OpenRouter returned no choices"))?
        .message
        .content
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if content.is_empty() {
        anyhow::bail!("OpenRouter returned empty content");
    }
    Ok(content)
}

/// Builds a readable error from an OpenRouter error body, which is usually
/// `{"error": {"message": "..."}}` but may be plain text.
fn describe_error_body(body: Option<&str>) -> String {
    let Some(body) = body else {
        return "no response body".to_string();
    };
    serde_json::from_str::<OpenRouterErrorBody>(body)
        .ok()
        .and_then(|parsed| parsed.error.map(|err| err.message))
        .unwrap_or_else(|| body.chars().take(200).collect())
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterErrorBody {
    error: Option<OpenRouterError>,
}

#[derive(Deserialize)]
struct OpenRouterError {
    message: String,
}

#[cfg(test)]
#[path = "openrouter_tests.rs"]
mod tests;
