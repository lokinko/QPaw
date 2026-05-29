use std::sync::Arc;

use crate::debug;
use crate::error::QPawResult;
use crate::llm::LlmClient;
use crate::models::{
    ExplicitMemoryItem, LayeredMemoryItem, MemoryLayer, MemoryLayerFilter, MemoryQueryRequest,
    MemoryQueryResponse, WorkingMemoryItem,
};
use crate::storage::DocumentStore;

use super::explicit::memory_matches_query;

pub struct MemoryRetriever {
    store: Arc<DocumentStore>,
    _llm: Arc<LlmClient>,
}

impl MemoryRetriever {
    pub fn new(store: Arc<DocumentStore>, llm: Arc<LlmClient>) -> Self {
        Self { store, _llm: llm }
    }

    pub async fn query(&self, request: MemoryQueryRequest) -> QPawResult<MemoryQueryResponse> {
        let limit = request.limit.unwrap_or(12).clamp(1, 50);
        debug::log(
            "memory:retriever:query",
            format!(
                "query_len={} layer={:?} category={:?} limit={limit}",
                request.query.chars().count(),
                request.layer,
                request.category
            ),
        );
        let filter = MemoryLayerFilter {
            layer: request.layer.clone(),
            category: request.category.clone(),
            query: Some(request.query.clone()),
            include_archived: false,
        };
        let mut items = self.store.list_layered_memory(filter).await?;
        items.sort_by(|a, b| score(&request.query, b).cmp(&score(&request.query, a)));
        items.truncate(limit);
        let context = build_layered_context(&items);
        debug::log(
            "memory:retriever:query",
            format!(
                "returned_items={} context_len={}",
                items.len(),
                context.len()
            ),
        );
        Ok(MemoryQueryResponse { items, context })
    }

    pub async fn context_for_chat(&self, message: &str) -> QPawResult<String> {
        debug::log(
            "memory:retriever:context_for_chat",
            format!("message_len={}", message.chars().count()),
        );
        let working = self.store.list_active_working_memory().await?;
        let selected_explicit = match self.store.list_active_explicit_memories().await {
            Ok(explicit) => select_explicit_memories(message, explicit),
            Err(error) => {
                debug::log(
                    "memory:retriever:context_for_chat",
                    format!("explicit_memory_error={error}"),
                );
                Vec::new()
            }
        };
        let mut selected = Vec::new();
        for layer in [
            MemoryLayer::L0,
            MemoryLayer::L1Concept,
            MemoryLayer::L1Relation,
            MemoryLayer::L2,
            MemoryLayer::L3,
        ] {
            let mut response = self
                .query(MemoryQueryRequest {
                    query: message.to_string(),
                    layer: Some(layer),
                    category: None,
                    limit: Some(4),
                })
                .await?;
            selected.append(&mut response.items);
        }
        selected.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        selected.truncate(16);
        let context = build_context(&selected_explicit, &working, &selected);
        debug::log(
            "memory:retriever:context_for_chat",
            format!(
                "working_items={} explicit_items={} selected_items={} context_len={}",
                working.len(),
                selected_explicit.len(),
                selected.len(),
                context.len()
            ),
        );
        Ok(context)
    }
}

fn build_layered_context(items: &[LayeredMemoryItem]) -> String {
    build_context(&[], &[], items)
}

fn score(query: &str, item: &LayeredMemoryItem) -> usize {
    let query = query.to_lowercase();
    if query.trim().is_empty() {
        return 1;
    }

    query
        .split_whitespace()
        .map(|term| {
            usize::from(item.title.to_lowercase().contains(term)) * 3
                + usize::from(item.summary.to_lowercase().contains(term)) * 2
                + item
                    .tags
                    .iter()
                    .filter(|tag| tag.to_lowercase().contains(term))
                    .count()
        })
        .sum()
}

fn select_explicit_memories(
    message: &str,
    mut items: Vec<ExplicitMemoryItem>,
) -> Vec<ExplicitMemoryItem> {
    let mut matched = items
        .iter()
        .filter(|item| memory_matches_query(item, message))
        .cloned()
        .collect::<Vec<_>>();
    if matched.is_empty() {
        items.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        matched = items.into_iter().take(3).collect();
    } else {
        matched.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        matched.truncate(6);
    }
    matched
}

fn build_context(
    explicit: &[ExplicitMemoryItem],
    working: &[WorkingMemoryItem],
    items: &[LayeredMemoryItem],
) -> String {
    let mut sections = Vec::new();
    if !explicit.is_empty() {
        sections.push("Immediate user memories:".to_string());
        sections.extend(
            explicit
                .iter()
                .map(|item| format!("- {} [{}]", item.body.trim(), item.keywords.join(", "))),
        );
    }
    if !working.is_empty() {
        sections.push("Today's working memory:".to_string());
        sections.extend(working.iter().map(|item| {
            format!(
                "- {:?} {}: {} [{}]",
                item.kind,
                item.title.trim(),
                item.summary.trim(),
                item.keywords.join(", ")
            )
        }));
    }
    if !items.is_empty() {
        sections.push("Layered long-term memory:".to_string());
    }
    sections.extend(
        items
            .iter()
            .map(|item| {
                format!(
                    "- {:?} {}: {}",
                    item.layer,
                    item.title.trim(),
                    item.summary.trim()
                )
            })
            .collect::<Vec<_>>(),
    );
    sections.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{build_context, select_explicit_memories};
    use crate::models::{ExplicitMemoryItem, ExplicitMemoryStatus};

    #[test]
    fn context_includes_immediate_user_memories_section() {
        let now = chrono::Utc::now();
        let explicit = vec![ExplicitMemoryItem {
            id: "explicit_1".to_string(),
            body: "记住我喜欢简洁回答".to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec!["简洁".to_string(), "回答".to_string()],
            created_at: now,
            last_used_at: now,
            status: ExplicitMemoryStatus::Active,
        }];

        let context = build_context(&explicit, &[], &[]);

        assert!(context.contains("Immediate user memories:"));
        assert!(context.contains("记住我喜欢简洁回答"));
    }

    #[test]
    fn select_explicit_memories_limits_matched_items_to_six_newest() {
        let now = chrono::Utc::now();
        let items = (0..7)
            .map(|index| {
                explicit_memory_item(index, "keyword", now + chrono::Duration::seconds(index))
            })
            .collect::<Vec<_>>();

        let selected = select_explicit_memories("keyword", items);

        assert_eq!(selected.len(), 6);
        assert_eq!(
            selected
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "explicit_6",
                "explicit_5",
                "explicit_4",
                "explicit_3",
                "explicit_2",
                "explicit_1"
            ]
        );
    }

    #[test]
    fn select_explicit_memories_falls_back_to_three_recent_when_no_match() {
        let now = chrono::Utc::now();
        let items = (0..4)
            .map(|index| {
                explicit_memory_item(
                    index,
                    "unrelated preference",
                    now + chrono::Duration::seconds(index),
                )
            })
            .collect::<Vec<_>>();

        let selected = select_explicit_memories("keyword", items);

        assert_eq!(selected.len(), 3);
        assert_eq!(
            selected
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["explicit_3", "explicit_2", "explicit_1"]
        );
    }

    fn explicit_memory_item(
        index: i64,
        body: &str,
        last_used_at: chrono::DateTime<chrono::Utc>,
    ) -> ExplicitMemoryItem {
        ExplicitMemoryItem {
            id: format!("explicit_{index}"),
            body: body.to_string(),
            source: "chat".to_string(),
            tags: vec!["explicit_memory_request".to_string()],
            keywords: vec![body.to_string()],
            created_at: last_used_at,
            last_used_at,
            status: ExplicitMemoryStatus::Active,
        }
    }
}
