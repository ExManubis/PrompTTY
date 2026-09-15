use std::path::Path;

use super::{build_messages, find_pr_template};
use crate::code_review::api::{GenerateCodeReviewContentRequest, OutputType};

fn request(output_type: OutputType) -> GenerateCodeReviewContentRequest {
    GenerateCodeReviewContentRequest {
        output_type,
        diff: "diff --git a/src/main.rs b/src/main.rs".to_string(),
        branch_name: "mikkel/add-auth".to_string(),
        commit_messages: vec!["feat(auth): implement JWT-based authentication".to_string()],
        pr_template: None,
    }
}

#[test]
fn commit_message_prompt_enforces_conventional_format() {
    let (system, user) = build_messages(&request(OutputType::CommitMessage));
    assert!(system.contains("never invent"));
    for expected in ["feat", "fix", "chore", "imperative mood", "72 characters"] {
        assert!(user.contains(expected), "prompt missing `{expected}`");
    }
    assert!(user.contains("Example: `feat: implement JWT-based authentication`"));
    assert!(user.contains("Never add a scope in parentheses"));
    assert!(user.contains("Branch name: mikkel/add-auth"));
    assert!(user.contains("diff --git a/src/main.rs"));
}

#[test]
fn pr_title_prompt_is_single_line_conventional_subject() {
    let (_, user) = build_messages(&request(OutputType::PrTitle));
    assert!(user.contains("pull request title"));
    assert!(user.contains("- feat(auth): implement JWT-based authentication"));
}

#[test]
fn pr_description_prompt_embeds_repo_template() {
    let mut req = request(OutputType::PrDescription);
    req.pr_template = Some("## Description\n\n## Testing\n\n- [ ] presubmit".to_string());
    let (_, user) = build_messages(&req);
    assert!(user.contains("Keep its section headings exactly as written"));
    assert!(user.contains("## Testing\n\n- [ ] presubmit"));
}

#[test]
fn pr_description_prompt_falls_back_to_default_structure() {
    let (system, user) = build_messages(&request(OutputType::PrDescription));
    assert!(user.contains("## Description"));
    assert!(user.contains("## Testing"));
    assert!(!system.contains("template"));
}

fn write_repo_file(repo: &Path, relative: &str, contents: &str) {
    let path = repo.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

#[test]
fn finds_github_pr_template() {
    let repo = tempfile::tempdir().unwrap();
    write_repo_file(
        repo.path(),
        ".github/pull_request_template.md",
        "## Description\n",
    );
    assert_eq!(
        find_pr_template(repo.path()).as_deref(),
        Some("## Description\n")
    );
}

#[test]
fn finds_first_template_alphabetically_in_directory_form() {
    let repo = tempfile::tempdir().unwrap();
    write_repo_file(repo.path(), ".github/PULL_REQUEST_TEMPLATE/bug.md", "Bug");
    write_repo_file(
        repo.path(),
        ".github/PULL_REQUEST_TEMPLATE/feature.md",
        "Feature",
    );
    assert_eq!(find_pr_template(repo.path()).as_deref(), Some("Bug"));
}

#[test]
fn returns_none_without_template() {
    let repo = tempfile::tempdir().unwrap();
    assert_eq!(find_pr_template(repo.path()), None);
}

#[test]
fn truncates_oversized_templates_on_char_boundary() {
    let repo = tempfile::tempdir().unwrap();
    write_repo_file(
        repo.path(),
        ".github/PULL_REQUEST_TEMPLATE.md",
        &"x".repeat(10_000),
    );
    let template = find_pr_template(repo.path()).unwrap();
    assert!(template.len() <= 4_000);
    assert!(std::str::from_utf8(template.as_bytes()).is_ok());
}
