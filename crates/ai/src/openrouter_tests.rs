use super::{completion_request_body, describe_error_body, parse_completion_content};

#[test]
fn request_body_has_expected_shape() {
    let body = completion_request_body("anthropic/claude-sonnet-4.5", "sys", "usr");
    assert_eq!(body["model"], "anthropic/claude-sonnet-4.5");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][0]["content"], "sys");
    assert_eq!(body["messages"][1]["role"], "user");
    assert_eq!(body["messages"][1]["content"], "usr");
    assert_eq!(body["stream"], false);
    assert!(body["max_tokens"].as_u64().unwrap() > 0);
}

#[test]
fn parses_completion_content() {
    let body = r#"{
        "id": "gen-1",
        "choices": [
            {"index": 0, "finish_reason": "stop",
             "message": {"role": "assistant", "content": "feat(core): add thing\n\nBody text."}}
        ]
    }"#;
    let content = parse_completion_content(body).unwrap();
    assert_eq!(content, "feat(core): add thing\n\nBody text.");
}

#[test]
fn trims_and_rejects_empty_content() {
    let body = r#"{"choices": [{"message": {"role": "assistant", "content": "   \n  "}}]}"#;
    assert!(parse_completion_content(body).is_err());

    let null_content = r#"{"choices": [{"message": {"role": "assistant", "content": null}}]}"#;
    assert!(parse_completion_content(null_content).is_err());
}

#[test]
fn rejects_response_without_choices() {
    assert!(parse_completion_content(r#"{"choices": []}"#).is_err());
    assert!(parse_completion_content("not json").is_err());
}

#[test]
fn describes_error_bodies() {
    let structured = r#"{"error": {"message": "Insufficient credits", "code": 402}}"#;
    assert_eq!(
        describe_error_body(Some(structured)),
        "Insufficient credits"
    );

    let plain = "Gateway timeout";
    assert_eq!(describe_error_body(Some(plain)), "Gateway timeout");

    // Long plain bodies are truncated to keep surfaced errors readable.
    let long = "x".repeat(500);
    assert_eq!(describe_error_body(Some(&long)).len(), 200);

    assert_eq!(describe_error_body(None), "no response body");
}
