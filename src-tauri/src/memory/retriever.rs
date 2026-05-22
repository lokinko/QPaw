use std::sync::Arc;

use crate::debug;
use crate::error::QPawResult;
use crate::llm::LlmClient;
use crate::models::{
    LayeredMemoryItem, MemoryLayer, MemoryLayerFilter, MemoryQueryRequest, MemoryQueryResponse,
};
use crate::storage::DocumentStore;

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
        let context = build_context(&working, &selected);
        debug::log(
            "memory:retriever:context_for_chat",
            format!(
                "working_items={} selected_items={} context_len={}",
                working.len(),
                selected.len(),
                context.len()
            ),
        );
        Ok(context)
    }
}

fn build_layered_context(items: &[LayeredMemoryItem]) -> String {
    build_context(&[], items)
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

fn build_context(
    working: &[crate::models::WorkingMemoryItem],
    items: &[LayeredMemoryItem],
) -> String {
    let mut sections = Vec::new();
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
