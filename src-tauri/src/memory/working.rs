use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};

use crate::debug;
use crate::error::QPawResult;
use crate::models::{InteractionEvent, WorkingMemoryItem, WorkingMemoryKind};
use crate::storage::DocumentStore;

pub struct WorkingMemoryUpdater {
    store: Arc<DocumentStore>,
}

impl WorkingMemoryUpdater {
    pub fn new(store: Arc<DocumentStore>) -> Self {
        Self { store }
    }

    pub async fn update_from_user_event(
        &self,
        event: &InteractionEvent,
        retention_hours: u64,
    ) -> QPawResult<Vec<WorkingMemoryItem>> {
        let mut updated = Vec::new();
        for draft in extract_drafts(&event.summary) {
            let item = self.upsert(event, draft, retention_hours).await?;
            updated.push(item);
        }
        debug::log(
            "memory:working:update_from_user_event",
            format!("event_id={} updated_count={}", event.id, updated.len()),
        );
        Ok(updated)
    }

    async fn upsert(
        &self,
        event: &InteractionEvent,
        draft: WorkingMemoryDraft,
        retention_hours: u64,
    ) -> QPawResult<WorkingMemoryItem> {
        let now = Utc::now();
        let expires_at = now + ChronoDuration::hours(retention_hours.max(1) as i64);
        let existing = self
            .store
            .list_active_working_memory()
            .await?
            .into_iter()
            .find(|item| same_memory(item, &draft));

        let mut item = existing.unwrap_or_else(|| WorkingMemoryItem {
            id: stable_working_id(&draft.kind, &draft.title),
            kind: draft.kind.clone(),
            title: draft.title.clone(),
            summary: String::new(),
            keywords: Vec::new(),
            source_event_ids: Vec::new(),
            confidence: draft.confidence,
            created_at: now,
            updated_at: now,
            expires_at,
        });

        item.kind = draft.kind;
        item.title = draft.title;
        item.summary = draft.summary;
        item.keywords = merge_strings(item.keywords, draft.keywords);
        item.source_event_ids = merge_strings(item.source_event_ids, vec![event.id.clone()]);
        item.confidence = item.confidence.max(draft.confidence).clamp(0.0, 1.0);
        item.updated_at = now;
        item.expires_at = expires_at;

        self.store.save_working_memory(&item).await?;
        Ok(item)
    }
}

#[derive(Debug, Clone)]
struct WorkingMemoryDraft {
    kind: WorkingMemoryKind,
    title: String,
    summary: String,
    keywords: Vec<String>,
    confidence: f64,
}

fn extract_drafts(message: &str) -> Vec<WorkingMemoryDraft> {
    let mut drafts = Vec::new();
    if let Some(name) = extract_identity_name(message) {
        drafts.push(WorkingMemoryDraft {
            kind: WorkingMemoryKind::Identity,
            title: "QPaw name".to_string(),
            summary: format!("QPaw should identify itself as {name}."),
            keywords: vec!["qpaw".to_string(), "name".to_string(), name],
            confidence: 0.95,
        });
    }

    drafts
}

fn extract_identity_name(message: &str) -> Option<String> {
    let normalized = message.trim();
    let patterns = [
        "你叫",
        "你名字是",
        "你的名字是",
        "QPaw 叫",
        "QPaw叫",
        "qpaw 叫",
        "qpaw叫",
        "它叫",
        "名字是",
    ];

    for pattern in patterns {
        if let Some(index) = normalized.find(pattern) {
            let start = index + pattern.len();
            return clean_name(&normalized[start..]);
        }
    }

    None
}

fn clean_name(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_start_matches(['：', ':', '“', '"', '\'', '叫'])
        .trim();
    let name = trimmed
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    '，' | ',' | '。' | '.' | '！' | '!' | '？' | '?' | '；' | ';' | '、'
                )
        })
        .next()
        .unwrap_or_default()
        .trim_matches(['”', '"', '\'', '。', '.'])
        .trim()
        .chars()
        .take(32)
        .collect::<String>();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn same_memory(item: &WorkingMemoryItem, draft: &WorkingMemoryDraft) -> bool {
    item.kind == draft.kind
        && (item.title.eq_ignore_ascii_case(&draft.title)
            || item.keywords.iter().any(|keyword| {
                draft
                    .keywords
                    .iter()
                    .any(|other| keyword.eq_ignore_ascii_case(other))
            }))
}

fn stable_working_id(kind: &WorkingMemoryKind, title: &str) -> String {
    match kind {
        WorkingMemoryKind::Identity => "working_identity_qpaw_name".to_string(),
        _ => format!(
            "working_{}_{}",
            format!("{kind:?}").to_lowercase(),
            title
                .to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .take(32)
                .collect::<String>()
        ),
    }
}

fn merge_strings(mut left: Vec<String>, right: Vec<String>) -> Vec<String> {
    for value in right {
        if !left
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&value))
        {
            left.push(value);
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn test_db_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("qpaw-{name}-{}", Uuid::new_v4()))
    }

    #[test]
    fn extracts_qpaw_name_from_chinese_sentence() {
        assert_eq!(extract_identity_name("你叫 yuki"), Some("yuki".to_string()));
        assert_eq!(
            extract_identity_name("它叫 yuki。"),
            Some("yuki".to_string())
        );
        assert_eq!(
            extract_identity_name("QPaw 叫 Yuki，以后这么回答"),
            Some("Yuki".to_string())
        );
    }

    #[test]
    fn ignores_sentence_without_identity_fact() {
        assert_eq!(extract_identity_name("今天记得提醒我喝水"), None);
    }

    #[tokio::test]
    async fn user_identity_fact_updates_working_memory() {
        let path = test_db_path("working-updater");
        {
            let store = Arc::new(DocumentStore::connect(path.clone()).await.unwrap());
            let updater = WorkingMemoryUpdater::new(Arc::clone(&store));
            let event = InteractionEvent {
                id: "event_1".to_string(),
                kind: crate::models::InteractionEventKind::ChatMessage,
                actor: "user".to_string(),
                summary: "你叫 yuki".to_string(),
                content: json!({ "content": "你叫 yuki" }),
                tags: vec!["chat".to_string()],
                created_at: Utc::now(),
            };

            let updated = updater.update_from_user_event(&event, 36).await.unwrap();
            let items = store.list_active_working_memory().await.unwrap();

            assert_eq!(updated.len(), 1);
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].kind, WorkingMemoryKind::Identity);
            assert!(items[0].summary.contains("yuki"));
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn repeated_identity_fact_updates_existing_item() {
        let path = test_db_path("working-updater-repeat");
        {
            let store = Arc::new(DocumentStore::connect(path.clone()).await.unwrap());
            let updater = WorkingMemoryUpdater::new(Arc::clone(&store));
            for (id, summary) in [("event_1", "你叫 yuki"), ("event_2", "你叫 momo")] {
                let event = InteractionEvent {
                    id: id.to_string(),
                    kind: crate::models::InteractionEventKind::ChatMessage,
                    actor: "user".to_string(),
                    summary: summary.to_string(),
                    content: json!({ "content": summary }),
                    tags: vec!["chat".to_string()],
                    created_at: Utc::now(),
                };
                updater.update_from_user_event(&event, 36).await.unwrap();
            }

            let items = store.list_active_working_memory().await.unwrap();

            assert_eq!(items.len(), 1);
            assert!(items[0].summary.contains("momo"));
            assert_eq!(items[0].source_event_ids.len(), 2);
        }
        let _ = std::fs::remove_dir_all(path);
    }
}
