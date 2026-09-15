//! Prompt construction for the code-review AI flows.
//!
//! Commit messages and PR titles follow the conventional-commit format the
//! repo uses (`type(scope): subject`); PR bodies fill the target repo's own
//! PR template so generated descriptions match the project's layout instead
//! of a generic structure.

use std::path::Path;

use crate::code_review::api::{GenerateCodeReviewContentRequest, OutputType};

/// Candidate locations for a repo's PR template, in GitHub's own resolution
/// order (GitHub falls back to the default branch's template; we only see the
/// working tree, so local files are all we have).
const PR_TEMPLATE_PATHS: [&str; 6] = [
    ".github/pull_request_template.md",
    ".github/PULL_REQUEST_TEMPLATE.md",
    "docs/PULL_REQUEST_TEMPLATE.md",
    "PULL_REQUEST_TEMPLATE.md",
    "pull_request_template.md",
    // Directory form: GitHub picks the first template alphabetically.
    ".github/PULL_REQUEST_TEMPLATE",
];

/// Reads the repo's PR template, if one exists. Returns the raw markdown
/// (truncated to a sane prompt budget) so the PR-description prompt can ask
/// the model to fill it in.
pub fn find_pr_template(repo_path: &Path) -> Option<String> {
    const MAX_TEMPLATE_BYTES: usize = 4_000;

    for candidate in PR_TEMPLATE_PATHS {
        let path = if candidate.ends_with("PULL_REQUEST_TEMPLATE") {
            let dir = repo_path.join(candidate);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut templates: Vec<_> = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
                .collect();
            templates.sort();
            templates.into_iter().next()
        } else {
            let path = repo_path.join(candidate);
            path.is_file().then_some(path)
        };

        let Some(path) = path else {
            continue;
        };
        if let Ok(mut template) = std::fs::read_to_string(&path) {
            template.truncate(template.floor_char_boundary(MAX_TEMPLATE_BYTES));
            if !template.trim().is_empty() {
                return Some(template);
            }
        }
    }
    None
}

/// Returns the `(system, user)` prompt pair for the request's output type.
pub fn build_messages(request: &GenerateCodeReviewContentRequest) -> (String, String) {
    let system = SYSTEM_PROMPT.to_string();
    let user = match request.output_type {
        OutputType::CommitMessage => commit_message_prompt(request),
        OutputType::PrTitle => pr_title_prompt(request),
        OutputType::PrDescription => pr_description_prompt(request),
    };
    (system, user)
}

const SYSTEM_PROMPT: &str = "\
You are an expert software engineer writing concise, factual git artifacts \
(commit messages, pull request titles and descriptions) from code diffs. \
Describe only what the diff shows; never invent features, files, or changes. \
Output plain text only, with no markdown code fences around your answer.";

fn diff_context(request: &GenerateCodeReviewContentRequest) -> String {
    let mut context = String::new();
    if !request.branch_name.is_empty() {
        context.push_str(&format!("Branch name: {}\n", request.branch_name));
    }
    if !request.commit_messages.is_empty() {
        context.push_str("Commit messages on this branch:\n");
        for message in &request.commit_messages {
            context.push_str(&format!("- {message}\n"));
        }
    }
    context.push_str("\nDiff:\n```diff\n");
    context.push_str(&request.diff);
    context.push_str("\n```");
    context
}

fn commit_message_prompt(request: &GenerateCodeReviewContentRequest) -> String {
    format!(
        "Write a commit message for the diff below.\n\
         \n\
         Use the conventional commit format without a scope:\n\
         \x20 <type>: <subject>\n\
         \n\
         - `type` is one of: feat, fix, chore, docs, style, refactor, perf, test.\n\
         - Never add a scope in parentheses after the type.\n\
         - The subject uses the imperative mood, starts lowercase, has no\n\
           trailing period, and stays within 72 characters.\n\
         - Add a blank line and a short body explaining why the change was\n\
           made when the diff alone doesn't make it obvious; wrap body lines\n\
           at 72 characters. Omit the body for trivial changes.\n\
         - Output exactly one commit message and nothing else.\n\
         \n\
         Example: `feat: implement JWT-based authentication`\n\
         \n\
         {}",
        diff_context(request)
    )
}

fn pr_title_prompt(request: &GenerateCodeReviewContentRequest) -> String {
    format!(
        "Write a pull request title for the diff below, which summarizes an \
         entire branch (the commit messages are provided as context).\n\
         \n\
         Use the conventional commit subject format:\n\
         \x20 <type>(<optional scope>): <subject>\n\
         \n\
         - `type` is one of: feat, fix, chore, docs, style, refactor, perf, test.\n\
         - The subject uses the imperative mood, starts lowercase, has no\n\
           trailing period, and stays within 72 characters.\n\
         - Output exactly one line and nothing else.\n\
         \n\
         Example: `feat(auth): implement JWT-based authentication`\n\
         \n\
         {}",
        diff_context(request)
    )
}

fn pr_description_prompt(request: &GenerateCodeReviewContentRequest) -> String {
    let template_section = match &request.pr_template {
        Some(template) => format!(
            "The repository uses this pull request template. Keep its section \
             headings exactly as written and fill in each section with content \
             derived from the diff; leave checkbox items as unchecked.\n\
             \n\
             ---\n\
             {template}\n\
             ---\n",
        ),
        // Mirrors this repo's own PR template so descriptions still have a
        // familiar structure when the target project has none.
        None => "\
         Structure the description as:\n\
         \n\
         ## Description\n\
         \x20 <what the change does and why>\n\
         \n\
         ## Testing\n\
         \x20 <how the change was or should be verified>\n"
            .to_string(),
    };

    format!(
        "Write a pull request description for the diff below, which summarizes \
         an entire branch (the commit messages are provided as context).\n\
         \n\
         {template_section}\n\
         \n\
         Write in markdown. Be concise and factual; describe only what the \
         diff shows. Output only the description text.\n\
         \n\
         {}",
        diff_context(request)
    )
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
