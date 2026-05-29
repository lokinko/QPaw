use super::{
    codex_timeout_message, llm_connectivity_timeout_message, CODEX_CONNECTIVITY_TEST_TIMEOUT,
    OPENAI_COMPATIBLE_CONNECTIVITY_TEST_TIMEOUT,
};
use std::time::Duration;

#[test]
fn codex_connectivity_test_timeout_is_sixty_seconds() {
    assert_eq!(CODEX_CONNECTIVITY_TEST_TIMEOUT, Duration::from_secs(60));
}

#[test]
fn openai_compatible_connectivity_test_timeout_stays_five_seconds() {
    assert_eq!(
        OPENAI_COMPATIBLE_CONNECTIVITY_TEST_TIMEOUT,
        Duration::from_secs(5)
    );
}

#[test]
fn codex_timeout_message_uses_supplied_limit() {
    assert_eq!(
        codex_timeout_message(Duration::from_secs(60)),
        "Codex CLI connectivity timed out after 60 seconds"
    );
}

#[test]
fn generic_connectivity_timeout_message_uses_supplied_limit() {
    assert_eq!(
        llm_connectivity_timeout_message(Duration::from_secs(5)),
        "LLM connectivity test timed out after 5 seconds"
    );
}
