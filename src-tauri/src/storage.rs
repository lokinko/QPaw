use std::path::PathBuf;

use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use surrealdb::engine::local::SurrealKv;
use surrealdb::Surreal;

mod records;

use records::*;

use crate::debug;
use crate::error::QPawResult;
use crate::models::{
    AppSettings, AvatarManifest, ChatMessage, ConsolidationStatus, HabitEvent, InteractionEvent,
    LayeredMemoryItem, MemoryConsolidationJob, MemoryDocument, MemoryL0, MemoryL1Concept,
    MemoryL1Relation, MemoryL2Event, MemoryL3Reflection, MemoryLayer, MemoryLayerFilter,
    MemoryStats, MemoryStatus, ReminderEvent, ReminderFeedbackPayload, WorkingMemoryItem,
};

pub struct DocumentStore {
    db: Surreal<surrealdb::engine::local::Db>,
}

impl DocumentStore {
    pub async fn connect(path: PathBuf) -> QPawResult<Self> {
        std::fs::create_dir_all(&path)?;
        let db_path = path.to_string_lossy().to_string();
        debug::log(
            "storage:connect",
            format!("opening surrealdb path={db_path}"),
        );
        let db = Surreal::new::<SurrealKv>(db_path).await?;
        db.use_ns("qpaw").use_db("local").await?;
        debug::log("storage:connect", "surrealdb namespace ready");
        let store = Self { db };
        store.clear_legacy_structured_memory_once().await?;
        Ok(store)
    }

    async fn clear_legacy_structured_memory_once(&self) -> QPawResult<()> {
        const MIGRATION_ID: &str = "clear_legacy_structured_memory_uid_v1";
        let existing: Option<SchemaMigrationRecord> =
            self.db.select(("schema_migration", MIGRATION_ID)).await?;
        if existing.is_some() {
            return Ok(());
        }

        debug::log(
            "storage:migration",
            "clearing legacy structured memory tables without uid",
        );
        self.db
            .query(
                "DELETE memory;
                 DELETE interaction_event;
                 DELETE working_memory;
                 DELETE memory_l0;
                 DELETE memory_l1_concept;
                 DELETE memory_l1_relation;
                 DELETE memory_l2_event;
                 DELETE memory_l3_reflection;
                 DELETE memory_consolidation_job;",
            )
            .await?;
        let _: Option<SchemaMigrationRecord> = self
            .db
            .create(("schema_migration", MIGRATION_ID))
            .content(SchemaMigrationRecord {
                applied_at: Utc::now(),
            })
            .await?;
        Ok(())
    }

    pub async fn get_settings(&self) -> QPawResult<AppSettings> {
        let settings: Option<AppSettings> = self.db.select(("app_settings", "default")).await?;
        debug::log(
            "storage:get_settings",
            format!("found_existing={}", settings.is_some()),
        );
        Ok(settings.unwrap_or_default())
    }

    pub async fn save_settings(&self, settings: &AppSettings) -> QPawResult<AppSettings> {
        debug::log(
            "storage:save_settings",
            format!(
                "llm_configured={} memory_enabled={} reminders_paused={}",
                !settings.llm.api_key.trim().is_empty() && !settings.llm.model.trim().is_empty(),
                settings.memory.enabled,
                settings.reminders.paused
            ),
        );
        let mut response = self
            .db
            .query("UPSERT app_settings:default CONTENT $settings;")
            .bind(("settings", settings.clone()))
            .await?;
        let saved: Option<AppSettings> = response.take(0)?;
        Ok(saved.unwrap_or_else(|| settings.clone()))
    }

    pub async fn save_avatar(&self, manifest: &AvatarManifest) -> QPawResult<()> {
        debug::log(
            "storage:save_avatar",
            format!(
                "avatar_id={} path_len={} kind={:?}",
                manifest.id,
                manifest.path.len(),
                manifest.kind
            ),
        );
        let _: Option<AvatarManifestRecord> = self
            .db
            .create("avatar")
            .content(AvatarManifestRecord::from(manifest))
            .await?;
        Ok(())
    }

    pub async fn append_chat(&self, message: &ChatMessage) -> QPawResult<()> {
        debug::log(
            "storage:append_chat",
            format!(
                "role={:?} content_len={}",
                message.role,
                message.content.chars().count()
            ),
        );
        let _: Option<ChatMessage> = self
            .db
            .create("conversation")
            .content(message.clone())
            .await?;
        Ok(())
    }

    pub async fn list_chat_history(&self) -> QPawResult<Vec<ChatMessage>> {
        let mut messages: Vec<ChatMessage> = self.db.select("conversation").await?;
        messages.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        debug::log(
            "storage:list_chat_history",
            format!("count={}", messages.len()),
        );
        Ok(messages)
    }

    pub async fn append_memory(&self, memory: &MemoryDocument) -> QPawResult<()> {
        debug::log(
            "storage:append_memory",
            format!(
                "source={} body_len={}",
                memory.source,
                memory.body.chars().count()
            ),
        );
        let _: Option<MemoryDocument> = self.db.create("memory").content(memory.clone()).await?;
        Ok(())
    }

    pub async fn list_memories(&self) -> QPawResult<Vec<MemoryDocument>> {
        let memories: Vec<MemoryDocument> = self.db.select("memory").await?;
        debug::log("storage:list_memories", format!("count={}", memories.len()));
        Ok(memories)
    }

    pub async fn clear_memory(&self) -> QPawResult<()> {
        debug::log(
            "storage:clear_memory",
            "deleting conversation and memory tables",
        );
        self.db
            .query(
                "DELETE conversation;
                 DELETE memory;
                 DELETE habit_event;
                 DELETE reminder_event;
                 DELETE interaction_event;
                 DELETE working_memory;
                 DELETE memory_l0;
                 DELETE memory_l1_concept;
                 DELETE memory_l1_relation;
                 DELETE memory_l2_event;
                 DELETE memory_l3_reflection;
                 DELETE memory_consolidation_job;",
            )
            .await?;
        Ok(())
    }

    pub async fn append_interaction_event(&self, event: &InteractionEvent) -> QPawResult<()> {
        debug::log(
            "storage:append_interaction_event",
            format!(
                "id={} kind={:?} actor={} summary_len={} tags={}",
                event.id,
                event.kind,
                event.actor,
                event.summary.chars().count(),
                event.tags.len()
            ),
        );
        let _: Option<InteractionEventRecord> = self
            .db
            .create("interaction_event")
            .content(InteractionEventRecord::try_from(event)?)
            .await?;
        Ok(())
    }

    pub async fn list_interaction_events(&self) -> QPawResult<Vec<InteractionEvent>> {
        let records: Vec<InteractionEventRecord> = self.db.select("interaction_event").await?;
        debug::log(
            "storage:list_interaction_events",
            format!("record_count={}", records.len()),
        );
        records
            .into_iter()
            .map(|record| InteractionEvent::try_from(record).map_err(Into::into))
            .collect()
    }

    pub async fn list_interaction_events_for_date(
        &self,
        date: NaiveDate,
    ) -> QPawResult<Vec<InteractionEvent>> {
        let events = self.list_interaction_events().await?;
        let filtered = events
            .into_iter()
            .filter(|event| event.created_at.date_naive() == date)
            .collect::<Vec<_>>();
        debug::log(
            "storage:list_interaction_events_for_date",
            format!("date={date} count={}", filtered.len()),
        );
        Ok(filtered)
    }

    pub async fn cleanup_raw_events(&self, retention_days: u64) -> QPawResult<()> {
        let cutoff = Utc::now() - ChronoDuration::days(retention_days as i64);
        debug::log(
            "storage:cleanup_raw_events",
            format!("retention_days={retention_days} cutoff={cutoff}"),
        );
        self.db
            .query("DELETE interaction_event WHERE created_at < $cutoff;")
            .bind(("cutoff", cutoff))
            .await?;
        Ok(())
    }

    pub async fn save_working_memory(&self, item: &WorkingMemoryItem) -> QPawResult<()> {
        debug::log(
            "storage:save_working_memory",
            format!(
                "id={} kind={:?} title_len={} keywords={}",
                item.id,
                item.kind,
                item.title.chars().count(),
                item.keywords.len()
            ),
        );
        self.delete_working_memory_by_id(&item.id).await?;
        let _: Option<WorkingMemoryRecord> = self
            .db
            .create("working_memory")
            .content(WorkingMemoryRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn list_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>> {
        let records: Vec<WorkingMemoryRecord> = self.db.select("working_memory").await?;
        let mut items = records
            .into_iter()
            .map(WorkingMemoryItem::from)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug::log(
            "storage:list_working_memory",
            format!("count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn list_active_working_memory(&self) -> QPawResult<Vec<WorkingMemoryItem>> {
        let now = Utc::now();
        let mut items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| item.expires_at > now)
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug::log(
            "storage:list_active_working_memory",
            format!("count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn list_working_memory_for_date(
        &self,
        date: NaiveDate,
    ) -> QPawResult<Vec<WorkingMemoryItem>> {
        let items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| {
                item.created_at.date_naive() == date || item.updated_at.date_naive() == date
            })
            .collect::<Vec<_>>();
        debug::log(
            "storage:list_working_memory_for_date",
            format!("date={date} count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn clear_working_memory(&self) -> QPawResult<()> {
        debug::log(
            "storage:clear_working_memory",
            "deleting all working memory",
        );
        self.db.query("DELETE working_memory;").await?;
        Ok(())
    }

    pub async fn clear_working_memory_for_date(&self, date: NaiveDate) -> QPawResult<()> {
        let items = self.list_working_memory_for_date(date).await?;
        debug::log(
            "storage:clear_working_memory_for_date",
            format!("date={date} count={}", items.len()),
        );
        for item in items {
            self.delete_working_memory_by_id(&item.id).await?;
        }
        Ok(())
    }

    pub async fn cleanup_expired_working_memory(&self) -> QPawResult<usize> {
        let now = Utc::now();
        let items = self
            .list_working_memory()
            .await?
            .into_iter()
            .filter(|item| item.expires_at <= now)
            .collect::<Vec<_>>();
        let count = items.len();
        for item in items {
            self.delete_working_memory_by_id(&item.id).await?;
        }
        debug::log(
            "storage:cleanup_expired_working_memory",
            format!("deleted={count}"),
        );
        Ok(count)
    }

    async fn delete_working_memory_by_id(&self, id: &str) -> QPawResult<()> {
        self.db
            .query("DELETE working_memory WHERE uid = $id;")
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn save_l0(&self, item: &MemoryL0) -> QPawResult<()> {
        debug::log(
            "storage:save_l0",
            format!(
                "id={} category={:?} title_len={}",
                item.id,
                item.category,
                item.title.len()
            ),
        );
        self.delete_memory_by_id(MemoryLayer::L0, &item.id).await?;
        let _: Option<MemoryL0Record> = self
            .db
            .create("memory_l0")
            .content(MemoryL0Record::from(item))
            .await?;
        Ok(())
    }

    pub async fn save_l1_concept(&self, item: &MemoryL1Concept) -> QPawResult<()> {
        debug::log(
            "storage:save_l1_concept",
            format!(
                "id={} name_len={} type={}",
                item.id,
                item.name.len(),
                item.concept_type
            ),
        );
        self.delete_memory_by_id(MemoryLayer::L1Concept, &item.id)
            .await?;
        let _: Option<MemoryL1ConceptRecord> = self
            .db
            .create("memory_l1_concept")
            .content(MemoryL1ConceptRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn save_l1_relation(&self, item: &MemoryL1Relation) -> QPawResult<()> {
        debug::log(
            "storage:save_l1_relation",
            format!(
                "id={} subject_len={} predicate={} object_len={}",
                item.id,
                item.subject.len(),
                item.predicate,
                item.object.len()
            ),
        );
        self.delete_memory_by_id(MemoryLayer::L1Relation, &item.id)
            .await?;
        let _: Option<MemoryL1RelationRecord> = self
            .db
            .create("memory_l1_relation")
            .content(MemoryL1RelationRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn save_l2(&self, item: &MemoryL2Event) -> QPawResult<()> {
        debug::log(
            "storage:save_l2",
            format!(
                "id={} title_len={} importance={}",
                item.id,
                item.title.len(),
                item.importance
            ),
        );
        self.delete_memory_by_id(MemoryLayer::L2, &item.id).await?;
        let _: Option<MemoryL2EventRecord> = self
            .db
            .create("memory_l2_event")
            .content(MemoryL2EventRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn save_l3(&self, item: &MemoryL3Reflection) -> QPawResult<()> {
        debug::log(
            "storage:save_l3",
            format!(
                "id={} kind={:?} title_len={}",
                item.id,
                item.kind,
                item.title.len()
            ),
        );
        self.delete_memory_by_id(MemoryLayer::L3, &item.id).await?;
        let _: Option<MemoryL3ReflectionRecord> = self
            .db
            .create("memory_l3_reflection")
            .content(MemoryL3ReflectionRecord::from(item))
            .await?;
        Ok(())
    }

    pub async fn list_l0(&self) -> QPawResult<Vec<MemoryL0>> {
        let records: Vec<MemoryL0Record> = self.db.select("memory_l0").await?;
        Ok(records.into_iter().map(MemoryL0::from).collect())
    }

    pub async fn list_l1_concepts(&self) -> QPawResult<Vec<MemoryL1Concept>> {
        let records: Vec<MemoryL1ConceptRecord> = self.db.select("memory_l1_concept").await?;
        Ok(records.into_iter().map(MemoryL1Concept::from).collect())
    }

    pub async fn list_l1_relations(&self) -> QPawResult<Vec<MemoryL1Relation>> {
        let records: Vec<MemoryL1RelationRecord> = self.db.select("memory_l1_relation").await?;
        Ok(records.into_iter().map(MemoryL1Relation::from).collect())
    }

    pub async fn list_l2(&self) -> QPawResult<Vec<MemoryL2Event>> {
        let records: Vec<MemoryL2EventRecord> = self.db.select("memory_l2_event").await?;
        Ok(records.into_iter().map(MemoryL2Event::from).collect())
    }

    pub async fn list_l3(&self) -> QPawResult<Vec<MemoryL3Reflection>> {
        let records: Vec<MemoryL3ReflectionRecord> = self.db.select("memory_l3_reflection").await?;
        Ok(records.into_iter().map(MemoryL3Reflection::from).collect())
    }

    pub async fn list_layered_memory(
        &self,
        filter: MemoryLayerFilter,
    ) -> QPawResult<Vec<LayeredMemoryItem>> {
        debug::log(
            "storage:list_layered_memory",
            format!(
                "layer={:?} category={:?} query_len={} include_archived={}",
                filter.layer,
                filter.category,
                filter.query.as_deref().unwrap_or_default().chars().count(),
                filter.include_archived
            ),
        );
        let mut items = Vec::new();

        if filter.layer.is_none() || filter.layer == Some(MemoryLayer::L0) {
            items.extend(
                self.list_l0()
                    .await?
                    .into_iter()
                    .map(LayeredMemoryItem::from),
            );
        }
        if filter.layer.is_none() || filter.layer == Some(MemoryLayer::L1Concept) {
            items.extend(
                self.list_l1_concepts()
                    .await?
                    .into_iter()
                    .map(LayeredMemoryItem::from),
            );
        }
        if filter.layer.is_none() || filter.layer == Some(MemoryLayer::L1Relation) {
            items.extend(
                self.list_l1_relations()
                    .await?
                    .into_iter()
                    .map(LayeredMemoryItem::from),
            );
        }
        if filter.layer.is_none() || filter.layer == Some(MemoryLayer::L2) {
            items.extend(
                self.list_l2()
                    .await?
                    .into_iter()
                    .map(LayeredMemoryItem::from),
            );
        }
        if filter.layer.is_none() || filter.layer == Some(MemoryLayer::L3) {
            items.extend(
                self.list_l3()
                    .await?
                    .into_iter()
                    .map(LayeredMemoryItem::from),
            );
        }

        if let Some(category) = filter.category {
            items.retain(|item| item.category.as_ref() == Some(&category));
        }
        if !filter.include_archived {
            items.retain(|item| item.status != MemoryStatus::Archived);
        }
        if let Some(query) = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            let terms = query.to_lowercase();
            items.retain(|item| {
                item.title.to_lowercase().contains(&terms)
                    || item.summary.to_lowercase().contains(&terms)
                    || item
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&terms))
            });
        }

        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        debug::log(
            "storage:list_layered_memory",
            format!("returned_count={}", items.len()),
        );
        Ok(items)
    }

    pub async fn delete_memory_by_id(&self, layer: MemoryLayer, id: &str) -> QPawResult<()> {
        let table = match layer {
            MemoryLayer::L0 => "memory_l0",
            MemoryLayer::L1Concept => "memory_l1_concept",
            MemoryLayer::L1Relation => "memory_l1_relation",
            MemoryLayer::L2 => "memory_l2_event",
            MemoryLayer::L3 => "memory_l3_reflection",
        };
        debug::log(
            "storage:delete_memory_by_id",
            format!("table={table} layer={layer:?} id={id}"),
        );
        self.db
            .query(format!("DELETE {table} WHERE uid = $id;"))
            .bind(("id", id.to_string()))
            .await?;
        Ok(())
    }

    pub async fn save_consolidation_job(&self, job: &MemoryConsolidationJob) -> QPawResult<()> {
        debug::log(
            "storage:save_consolidation_job",
            format!(
                "id={} date={} status={:?} input_events={} created={} updated={} archived={} error={}",
                job.id,
                job.date,
                job.status,
                job.input_event_count,
                job.created_count,
                job.updated_count,
                job.archived_count,
                job.error.is_some()
            ),
        );
        self.db
            .query("DELETE memory_consolidation_job WHERE uid = $id;")
            .bind(("id", job.id.clone()))
            .await?;
        let _: Option<MemoryConsolidationJobRecord> = self
            .db
            .create("memory_consolidation_job")
            .content(MemoryConsolidationJobRecord::from(job))
            .await?;
        Ok(())
    }

    pub async fn list_consolidation_jobs(&self) -> QPawResult<Vec<MemoryConsolidationJob>> {
        let records: Vec<MemoryConsolidationJobRecord> =
            self.db.select("memory_consolidation_job").await?;
        debug::log(
            "storage:list_consolidation_jobs",
            format!("count={}", records.len()),
        );
        Ok(records
            .into_iter()
            .map(MemoryConsolidationJob::from)
            .collect())
    }

    pub async fn consolidation_job_for_date(
        &self,
        date: NaiveDate,
    ) -> QPawResult<Option<MemoryConsolidationJob>> {
        Ok(self
            .list_consolidation_jobs()
            .await?
            .into_iter()
            .filter(|job| job.date == date)
            .max_by_key(|job| job.started_at))
    }

    pub async fn pending_consolidation_dates(&self) -> QPawResult<Vec<NaiveDate>> {
        let mut dates = self
            .list_consolidation_jobs()
            .await?
            .into_iter()
            .filter(|job| {
                matches!(
                    job.status,
                    ConsolidationStatus::Pending
                        | ConsolidationStatus::Failed
                        | ConsolidationStatus::Running
                )
            })
            .map(|job| job.date)
            .collect::<Vec<_>>();
        dates.sort();
        dates.dedup();
        Ok(dates)
    }

    pub async fn archive_stale_reflections(
        &self,
        decay_after_days: u64,
        archive_after_days: u64,
    ) -> QPawResult<usize> {
        let now = Utc::now();
        let decay_cutoff = now - ChronoDuration::days(decay_after_days as i64);
        let archive_cutoff = now - ChronoDuration::days(archive_after_days as i64);
        let mut changed = 0;

        for mut item in self.list_l3().await? {
            let next_status = if item.last_touched_at < archive_cutoff {
                MemoryStatus::Archived
            } else if item.last_touched_at < decay_cutoff {
                MemoryStatus::Decayed
            } else {
                item.status.clone()
            };

            if item.status != next_status {
                item.status = next_status;
                item.updated_at = now;
                self.save_l3(&item).await?;
                changed += 1;
            }
        }

        Ok(changed)
    }

    pub async fn memory_stats(&self) -> QPawResult<MemoryStats> {
        debug::log("storage:memory_stats", "calculating memory stats");
        let jobs = self.list_consolidation_jobs().await?;
        let pending_jobs = jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.status,
                    ConsolidationStatus::Pending
                        | ConsolidationStatus::Failed
                        | ConsolidationStatus::Running
                )
            })
            .count();
        let last_consolidation_at = jobs
            .iter()
            .filter(|job| job.status == ConsolidationStatus::Completed)
            .filter_map(|job| job.finished_at)
            .max();

        Ok(MemoryStats {
            raw_events: self.list_interaction_events().await?.len(),
            l0: self.list_l0().await?.len(),
            l1_concepts: self.list_l1_concepts().await?.len(),
            l1_relations: self.list_l1_relations().await?.len(),
            l2_events: self.list_l2().await?.len(),
            l3_reflections: self.list_l3().await?.len(),
            pending_jobs,
            last_consolidation_at,
        })
    }

    pub async fn append_habit_event(&self, active: bool, idle_seconds: u64) -> QPawResult<()> {
        debug::log(
            "storage:append_habit_event",
            format!("active={active} idle_seconds={idle_seconds}"),
        );
        let event = HabitEvent {
            active,
            idle_seconds,
            created_at: Utc::now(),
        };
        let _: Option<HabitEvent> = self.db.create("habit_event").content(event).await?;
        Ok(())
    }

    pub async fn append_reminder_event(&self, event: &ReminderEvent) -> QPawResult<()> {
        debug::log(
            "storage:append_reminder_event",
            format!(
                "reminder_id={} kind={:?} idle_seconds={}",
                event.reminder_id, event.kind, event.idle_seconds
            ),
        );
        let _: Option<ReminderEvent> = self
            .db
            .create("reminder_event")
            .content(event.clone())
            .await?;
        Ok(())
    }

    pub async fn set_reminder_feedback(&self, payload: &ReminderFeedbackPayload) -> QPawResult<()> {
        debug::log(
            "storage:set_reminder_feedback",
            format!(
                "reminder_id={} kind={:?} feedback={:?}",
                payload.reminder_id, payload.kind, payload.feedback
            ),
        );
        self.db
            .query(
                "UPDATE reminder_event
                 SET feedback = $feedback
                 WHERE reminder_id = $reminder_id;",
            )
            .bind(("feedback", payload.feedback.clone()))
            .bind(("reminder_id", payload.reminder_id.clone()))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::models::{
        ChatRole, InteractionEventKind, MemoryL0Category, WorkingMemoryItem, WorkingMemoryKind,
    };

    fn test_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("qpaw-{name}-{}", Uuid::new_v4()))
    }

    #[tokio::test]
    async fn interaction_event_round_trips_with_business_id() {
        let path = test_db_path("interaction-event");
        {
            let store = DocumentStore::connect(path.clone()).await.unwrap();
            let event = InteractionEvent {
                id: "event_1".to_string(),
                kind: InteractionEventKind::ChatMessage,
                actor: "user".to_string(),
                summary: "hello".to_string(),
                content: json!({ "content": "hello" }),
                tags: vec!["chat".to_string()],
                created_at: Utc::now(),
            };

            store.append_interaction_event(&event).await.unwrap();
            let events = store.list_interaction_events().await.unwrap();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].id, event.id);
            assert_eq!(events[0].summary, event.summary);
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn chat_history_round_trips_in_created_order() {
        let path = test_db_path("chat-history");
        {
            let store = DocumentStore::connect(path.clone()).await.unwrap();
            let first = ChatMessage {
                role: ChatRole::User,
                content: "hello".to_string(),
                created_at: Utc::now(),
            };
            let second = ChatMessage {
                role: ChatRole::Assistant,
                content: "hi".to_string(),
                created_at: first.created_at + ChronoDuration::seconds(1),
            };

            store.append_chat(&second).await.unwrap();
            store.append_chat(&first).await.unwrap();
            let history = store.list_chat_history().await.unwrap();

            assert_eq!(history.len(), 2);
            assert_eq!(history[0].content, first.content);
            assert_eq!(history[1].content, second.content);
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn layered_memory_round_trips_with_business_id() {
        let path = test_db_path("layered-memory");
        {
            let store = DocumentStore::connect(path.clone()).await.unwrap();
            let now = Utc::now();
            let memory = MemoryL0 {
                id: "l0_1".to_string(),
                category: MemoryL0Category::Preference,
                title: "reply style".to_string(),
                summary: "User prefers concise replies.".to_string(),
                tags: vec!["style".to_string()],
                confidence: 0.8,
                evidence_event_ids: vec!["event_1".to_string()],
                status: MemoryStatus::Active,
                created_at: now,
                updated_at: now,
                last_touched_at: now,
            };

            store.save_l0(&memory).await.unwrap();
            let memories = store.list_l0().await.unwrap();

            assert_eq!(memories.len(), 1);
            assert_eq!(memories[0].id, memory.id);
            assert_eq!(memories[0].title, memory.title);
        }
        let _ = std::fs::remove_dir_all(path);
    }

    #[tokio::test]
    async fn working_memory_round_trips_with_business_id() {
        let path = test_db_path("working-memory");
        {
            let store = DocumentStore::connect(path.clone()).await.unwrap();
            let now = Utc::now();
            let item = WorkingMemoryItem {
                id: "working_identity_qpaw_name".to_string(),
                kind: WorkingMemoryKind::Identity,
                title: "QPaw name".to_string(),
                summary: "QPaw should identify itself as yuki.".to_string(),
                keywords: vec!["qpaw".to_string(), "name".to_string(), "yuki".to_string()],
                source_event_ids: vec!["event_1".to_string()],
                confidence: 0.95,
                created_at: now,
                updated_at: now,
                expires_at: now + ChronoDuration::hours(36),
            };

            store.save_working_memory(&item).await.unwrap();
            let items = store.list_active_working_memory().await.unwrap();

            assert_eq!(items.len(), 1);
            assert_eq!(items[0].id, item.id);
            assert_eq!(items[0].summary, item.summary);
        }
        let _ = std::fs::remove_dir_all(path);
    }
}
