//! AI-generated copy for code-review flows (commit messages, PR titles and
//! bodies): the shared request/response wire types and the backend-agnostic
//! generation trait.

use async_trait::async_trait;

use crate::code_review::api::{
    GenerateCodeReviewContentRequest, GenerateCodeReviewContentResponse,
};

pub mod api;
pub mod prompts;

/// Generates AI copy for code-review flows: commit messages at dialog-open
/// time and PR titles / bodies at confirm time. `output_type` in the request
/// picks which of the three the backend returns.
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait CodeReviewAi: Send + Sync {
    async fn generate_code_review_content(
        &self,
        request: GenerateCodeReviewContentRequest,
    ) -> Result<GenerateCodeReviewContentResponse, anyhow::Error>;
}
