//! Resolves the AI backend for code-review content generation (commit
//! messages, PR titles and bodies) to OpenRouter when the user configured an
//! API key. Without a key, generation is skipped and callers fall back to
//! their non-AI paths (the `gh pr create --fill` fallback, the manual-type
//! placeholder).

use std::sync::Arc;

use ai::api_keys::ApiKeyManager;
use ai::code_review::CodeReviewAi;
use ai::openrouter::OpenRouterClient;
use warpui::{AppContext, SingletonEntity};

use crate::settings::ai::AISettings;

/// Returns the code-review AI backend when an OpenRouter key is configured,
/// or `None` to skip AI generation entirely.
pub fn resolve_code_review_ai(app: &AppContext) -> Option<Arc<dyn CodeReviewAi>> {
    let key = ApiKeyManager::as_ref(app)
        .keys()
        .open_router
        .clone()
        .filter(|key| !key.trim().is_empty())?;
    let model = AISettings::as_ref(app).openrouter_code_review_model();
    Some(Arc::new(OpenRouterClient::new(key, model)))
}
