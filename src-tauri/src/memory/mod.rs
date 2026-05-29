pub mod consolidator;
pub mod explicit;
pub mod prompts;
pub mod retriever;
pub mod store;
pub mod working;

use std::sync::Arc;
use std::time::Duration;

use chrono::{Local, NaiveDate, Utc};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::debug;
use crate::error::QPawResult;
use crate::llm::LlmClient;
use crate::models::{
    InteractionEvent, InteractionEventKind, LayeredMemoryItem, MemoryConsolidationReport,
    MemoryLayer, MemoryLayerFilter, MemoryQueryRequest, MemoryQueryResponse, MemoryStats,
    WorkingMemoryItem,
};
use crate::storage::DocumentStore;

use self::consolidator::MemoryConsolidator;
use self::explicit::explicit_memory_from_message;
use self::retriever::MemoryRetriever;
use self::working::WorkingMemoryUpdater;

pub struct MemoryService {
    store: Arc<DocumentStore>,
    consolidator: MemoryConsolidator,
    retriever: MemoryRetriever,
    working: WorkingMemoryUpdater,
    last_tick_date: Mutex<Option<NaiveDate>>,
}

impl MemoryService {
    pub fn new(store: Arc<DocumentStore>, llm: Arc<LlmClient>) -> Self {
        Self {
            consolidator: MemoryConsolidator::new(Arc::clone(&store), Arc::clone(&llm)),
            retriever: MemoryRetriever::new(Arc::clone(&store), llm),
            working: WorkingMemoryUpdater::new(Arc::clone(&store)),
            store,
            last_tick_date: Mutex::new(None),
        }
    }

    pub async fn record_event(
        &self,
        kind: InteractionEventKind,
        actor: impl Into<String>,
        summary: impl Into<String>,
        content: Value,
        tags: Vec<String>,
    ) -> QPawResult<InteractionEvent> {
        let event = InteractionEvent {
            id: Uuid::new_v4().to_string(),
            kind,
            actor: actor.into(),
            summary: summary.into(),
            content,
            tags,
            created_at: Utc::now(),
        };
        debug::log(
            "memory:record_event",
            format!(
                "id={} kind={:?} actor={} summary_len={} tags={}",
                event.id,
                event.kind,
                event.actor,
                event.summary.chars().count(),
                event.tags.len()
            ),
        );
        self.store.append_interaction_event(&event).await?;
        Ok(event)
    }

    pub async fn update_working_memory_from_user_event(
        &self,
        event: &InteractionEvent,
        retention_hours: u64,
    ) -> QPawResult<Vec<WorkingMemoryItem>> {
        self.working
            .update_from_user_event(event, retention_hours)
            .await
    }

    pub async fn query(&self, request: MemoryQueryRequest) -> QPawResult<MemoryQueryResponse> {
        debug::log(
            "memory:query",
            format!(
                "query_len={} layer={:?} category={:?} limit={:?}",
                request.query.chars().count(),
                request.layer,
                request.category,
                request.limit
            ),
        );
        self.retriever.query(request).await
    }

    pub async fn context_for_chat(&self, message: &str) -> QPawResult<String> {
        debug::log(
            "memory:context_for_chat",
            format!("message_len={}", message.chars().count()),
        );
        self.retriever.context_for_chat(message).await
    }

    pub async fn import_explicit_memory(
        &self,
        body: &str,
        source: &str,
        tags: Vec<String>,
    ) -> QPawResult<crate::models::ExplicitMemoryItem> {
        let item = explicit_memory_from_message(body, source, tags);
        debug::log(
            "memory:import_explicit_memory",
            format!("id={} body_len={}", item.id, item.body.chars().count()),
        );
        self.store.upsert_explicit_memory(&item).await?;
        Ok(item)
    }

    pub async fn list(&self, filter: MemoryLayerFilter) -> QPawResult<Vec<LayeredMemoryItem>> {
        debug::log(
            "memory:list",
            format!(
                "layer={:?} category={:?} query_len={} include_archived={}",
                filter.layer,
                filter.category,
                filter.query.as_deref().unwrap_or_default().chars().count(),
                filter.include_archived
            ),
        );
        self.store.list_layered_memory(filter).await
    }

    pub async fn delete(&self, layer: MemoryLayer, id: String) -> QPawResult<()> {
        debug::log("memory:delete", format!("layer={layer:?} id={id}"));
        self.store.delete_memory_by_id(layer, &id).await
    }

    pub async fn consolidate_date(
        &self,
        date: Option<NaiveDate>,
    ) -> QPawResult<MemoryConsolidationReport> {
        let date = date.unwrap_or_else(|| Local::now().date_naive());
        debug::log("memory:consolidate_date", format!("date={date}"));
        self.consolidator.run(date).await
    }

    pub async fn stats(&self) -> QPawResult<MemoryStats> {
        debug::log("memory:stats", "loading stats");
        self.store.memory_stats().await
    }

    pub async fn list_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>> {
        self.store.list_active_working_memory().await
    }

    pub async fn clear_working_memory(&self) -> QPawResult<()> {
        self.store.clear_working_memory().await
    }

    pub async fn run_startup_backfill(&self) {
        debug::log("memory:startup_backfill", "starting");
        if let Err(error) = self.consolidator.run_backfill().await {
            debug::err("memory:startup_backfill", format!("failed: {error}"));
        } else {
            debug::log("memory:startup_backfill", "finished");
        }
        if let Err(error) = self.store.cleanup_expired_working_memory().await {
            debug::err(
                "memory:startup_backfill",
                format!("working memory cleanup failed: {error}"),
            );
        }
    }

    async fn tick_midnight(&self) {
        let local_now = Local::now();
        let today = local_now.date_naive();
        let is_midnight = local_now.format("%H:%M").to_string() == "00:00";
        if !is_midnight {
            return;
        }

        let mut last_tick_date = self.last_tick_date.lock().await;
        if *last_tick_date == Some(today) {
            return;
        }
        *last_tick_date = Some(today);

        let yesterday = today.pred_opt().unwrap_or(today);
        debug::log(
            "memory:tick_midnight",
            format!("running consolidation date={yesterday}"),
        );
        if let Err(error) = self.consolidator.run(yesterday).await {
            debug::err(
                "memory:tick_midnight",
                format!("midnight memory consolidation failed: {error}"),
            );
        }
    }
}

pub fn start_memory_loop(memory: Arc<MemoryService>) {
    debug::log("memory:loop", "starting memory loop");
    tauri::async_runtime::spawn(async move {
        memory.run_startup_backfill().await;
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            memory.tick_midnight().await;
        }
    });
}
