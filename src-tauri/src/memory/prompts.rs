use chrono::NaiveDate;

use crate::models::{InteractionEvent, LayeredMemoryItem, WorkingMemoryItem};

pub fn consolidation_system_prompt() -> &'static str {
    "You are QPaw's local memory consolidator. Return strict JSON only. \
     Privacy mode is minimal: do not infer app names, window titles, keystrokes, or unseen context. \
     Consolidate repeated or similar facts, prefer recent evidence, and keep memories short."
}

pub fn consolidation_user_prompt(
    date: NaiveDate,
    events: &[InteractionEvent],
    working: &[WorkingMemoryItem],
    existing: &[LayeredMemoryItem],
) -> String {
    let events_json = serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".to_string());
    let working_json = serde_json::to_string_pretty(working).unwrap_or_else(|_| "[]".to_string());
    let existing_json = serde_json::to_string_pretty(existing).unwrap_or_else(|_| "[]".to_string());
    format!(
        "Date: {date}\n\
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
