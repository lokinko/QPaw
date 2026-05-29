use chrono::NaiveDate;

use crate::models::{ExplicitMemoryItem, InteractionEvent, LayeredMemoryItem, WorkingMemoryItem};

pub fn consolidation_system_prompt() -> &'static str {
    "You are QPaw's local memory consolidator. Return strict JSON only. \
     Privacy mode is minimal: do not infer app names, window titles, keystrokes, or unseen context. \
     Consolidate repeated or similar facts, prefer recent evidence, and keep memories short."
}

pub fn consolidation_user_prompt(
    date: NaiveDate,
    events: &[InteractionEvent],
    working: &[WorkingMemoryItem],
    explicit: &[ExplicitMemoryItem],
    existing: &[LayeredMemoryItem],
) -> String {
    let events_json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".to_string());
    let working_json = serde_json::to_string_pretty(working).unwrap_or_else(|_| "[]".to_string());
    let existing_json = serde_json::to_string_pretty(existing).unwrap_or_else(|_| "[]".to_string());
    let explicit_json = serde_json::to_string_pretty(explicit).unwrap_or_else(|_| "[]".to_string());
    format!(
        "Date: {date}\n\
         Explicit immediate memories:\n{explicit_json}\n\n\
         Existing memories:\n{existing_json}\n\n\
         Today's working memory:\n{working_json}\n\n\
         Raw interaction events:\n{events_json}\n\n\
         Return this JSON object exactly, with arrays omitted only when empty:\n\
         {{\n\
           \"l0\": [{{\"category\":\"preference|person_relation|task_project|health_habit|interaction_style|lesson\",\"title\":\"...\",\"summary\":\"...\",\"tags\":[\"...\"],\"confidence\":0.0,\"evidence_event_ids\":[\"...\"]}}],\n\
           \"l1_concepts\": [{{\"name\":\"...\",\"concept_type\":\"person|project|preference|habit|topic|task\",\"aliases\":[\"...\"],\"summary\":\"...\",\"tags\":[\"...\"],\"confidence\":0.0,\"evidence_event_ids\":[\"...\"]}}],\n\
           \"l1_relations\": [{{\"subject\":\"...\",\"predicate\":\"...\",\"object\":\"...\",\"summary\":\"...\",\"tags\":[\"...\"],\"confidence\":0.0,\"evidence_event_ids\":[\"...\"]}}],\n\
           \"l2_events\": [{{\"title\":\"...\",\"summary\":\"...\",\"entity_ids\":[\"...\"],\"tags\":[\"...\"],\"importance\":0.0,\"source_event_ids\":[\"...\"]}}],\n\
           \"l3_reflections\": [{{\"kind\":\"success|failure|observation\",\"title\":\"...\",\"insight\":\"...\",\"application\":\"...\",\"tags\":[\"...\"],\"confidence\":0.0,\"evidence_event_ids\":[\"...\"]}}],\n\
           \"archive_ids\": [{{\"layer\":\"l0|l1_concept|l1_relation|l2|l3\",\"id\":\"...\"}}]\n\
         }}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidation_prompt_includes_explicit_memories() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 5, 28).unwrap();
        let now = chrono::Utc::now();
        let explicit = vec![crate::models::ExplicitMemoryItem {
            id: "explicit_1".to_string(),
            body: "记住我喜欢简洁回答".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: now,
            last_used_at: now,
            status: crate::models::ExplicitMemoryStatus::Active,
        }];

        let prompt = consolidation_user_prompt(date, &[], &[], &explicit, &[]);

        assert!(prompt.contains("Explicit immediate memories"));
        assert!(prompt.contains("记住我喜欢简洁回答"));
    }

    #[test]
    fn consolidation_prompt_renders_empty_explicit_memories_as_json_array() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 5, 28).unwrap();

        let prompt = consolidation_user_prompt(date, &[], &[], &[], &[]);

        assert!(prompt.contains("Explicit immediate memories:\n[]\n\nExisting memories:"));
    }

    #[test]
    fn consolidation_prompt_json_escapes_explicit_memory_body() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 5, 28).unwrap();
        let now = chrono::Utc::now();
        let explicit = vec![crate::models::ExplicitMemoryItem {
            id: "explicit_1".to_string(),
            body: "line one\n{\"danger\":\"shape\"}".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["shape".to_string()],
            created_at: now,
            last_used_at: now,
            status: crate::models::ExplicitMemoryStatus::Active,
        }];

        let prompt = consolidation_user_prompt(date, &[], &[], &explicit, &[]);

        assert!(prompt.contains(r#""body": "line one\n{\"danger\":\"shape\"}""#));
        assert!(!prompt.contains("line one\n{\"danger\":\"shape\"}"));
    }
}
