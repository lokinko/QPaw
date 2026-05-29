use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use chrono::{Local, NaiveDate, Utc};
use serde::Deserialize;

use crate::debug;
use crate::error::{QPawError, QPawResult};
use crate::llm::LlmClient;
use crate::models::{
    ConsolidationStatus, MemoryConsolidationJob, MemoryConsolidationReport, MemoryL0,
    MemoryL0Category, MemoryL1Concept, MemoryL1Relation, MemoryL2Event, MemoryL3Reflection,
    MemoryLayerFilter, MemoryStatus, ReflectionKind,
};
use crate::storage::DocumentStore;

use super::prompts::{consolidation_system_prompt, consolidation_user_prompt};
use super::store::MemoryStore;

pub struct MemoryConsolidator {
    store: Arc<DocumentStore>,
    memory_store: MemoryStore,
    llm: Arc<LlmClient>,
}

#[derive(Debug, Deserialize, Default)]
struct ConsolidationOutput {
    #[serde(default)]
    l0: Vec<L0Draft>,
    #[serde(default)]
    l1_concepts: Vec<L1ConceptDraft>,
    #[serde(default)]
    l1_relations: Vec<L1RelationDraft>,
    #[serde(default)]
    l2_events: Vec<L2EventDraft>,
    #[serde(default)]
    l3_reflections: Vec<L3ReflectionDraft>,
    #[serde(default)]
    archive_ids: Vec<ArchiveDraft>,
}

#[derive(Debug, Deserialize)]
struct L0Draft {
    category: String,
    title: String,
    summary: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    evidence_event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct L1ConceptDraft {
    name: String,
    concept_type: String,
    #[serde(default)]
    aliases: Vec<String>,
    summary: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    evidence_event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct L1RelationDraft {
    subject: String,
    predicate: String,
    object: String,
    summary: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    evidence_event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct L2EventDraft {
    title: String,
    summary: String,
    #[serde(default)]
    entity_ids: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_confidence")]
    importance: f64,
    #[serde(default)]
    source_event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct L3ReflectionDraft {
    kind: String,
    title: String,
    insight: String,
    application: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    evidence_event_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArchiveDraft {
    layer: String,
    id: String,
}

fn default_confidence() -> f64 {
    0.6
}

impl MemoryConsolidator {
    pub fn new(store: Arc<DocumentStore>, llm: Arc<LlmClient>) -> Self {
        Self {
            memory_store: MemoryStore::new(Arc::clone(&store)),
            store,
            llm,
        }
    }

    pub async fn run(&self, date: NaiveDate) -> QPawResult<MemoryConsolidationReport> {
        debug::log("memory:consolidator:run", format!("date={date} start"));
        let settings = self.store.get_settings().await?;
        if !settings.memory.enabled {
            debug::log(
                "memory:consolidator:run",
                format!("date={date} skipped disabled"),
            );
            let job = self.pending_job(date, 0, Some("memory consolidation disabled".to_string()));
            self.store.save_consolidation_job(&job).await?;
            return Ok(MemoryConsolidationReport {
                job,
                message: "记忆沉淀已关闭".to_string(),
            });
        }

        let events = self.store.list_interaction_events_for_date(date).await?;
        let input_event_count = events.len();
        debug::log(
            "memory:consolidator:run",
            format!("date={date} input_event_count={input_event_count}"),
        );
        let mut job = self.running_job(date, input_event_count);
        self.store.save_consolidation_job(&job).await?;

        if events.is_empty() {
            debug::log(
                "memory:consolidator:run",
                format!("date={date} completed empty"),
            );
            job.status = ConsolidationStatus::Completed;
            job.finished_at = Some(Utc::now());
            self.store.save_consolidation_job(&job).await?;
            return Ok(MemoryConsolidationReport {
                job,
                message: "当天没有可沉淀的交互事件".to_string(),
            });
        }

        let existing = self
            .memory_store
            .list(MemoryLayerFilter {
                include_archived: false,
                ..MemoryLayerFilter::default()
            })
            .await?;
        let working = self.store.list_working_memory_for_date(date).await?;
        let explicit = match self.store.list_active_explicit_memories().await {
            Ok(explicit) => explicit,
            Err(error) => {
                debug::log(
                    "memory:consolidator:run",
                    format!("date={date} explicit_memory_error={error}"),
                );
                Vec::new()
            }
        };
        debug::log(
            "memory:consolidator:run",
            format!(
                "date={date} existing_memory_count={} working_memory_count={} explicit_memory_count={}",
                existing.len(),
                working.len(),
                explicit.len()
            ),
        );
        let user_prompt = consolidation_user_prompt(date, &events, &working, &explicit, &existing);
        let output = match self
            .llm
            .complete_json(&settings, consolidation_system_prompt(), &user_prompt)
            .await
        {
            Ok(content) => match parse_output(&content) {
                Ok(output) => {
                    debug::log(
                        "memory:consolidator:run",
                        format!(
                            "date={date} llm_json_parsed l0={} l1c={} l1r={} l2={} l3={} archive={}",
                            output.l0.len(),
                            output.l1_concepts.len(),
                            output.l1_relations.len(),
                            output.l2_events.len(),
                            output.l3_reflections.len(),
                            output.archive_ids.len()
                        ),
                    );
                    output
                }
                Err(error) => {
                    debug::err(
                        "memory:consolidator:run",
                        format!("date={date} json_parse_failed: {error}"),
                    );
                    job.status = ConsolidationStatus::Failed;
                    job.error = Some(error.to_string());
                    job.finished_at = Some(Utc::now());
                    self.store.save_consolidation_job(&job).await?;
                    return Ok(MemoryConsolidationReport {
                        job,
                        message: "LLM 返回的记忆 JSON 无法解析".to_string(),
                    });
                }
            },
            Err(error) => {
                debug::err(
                    "memory:consolidator:run",
                    format!("date={date} llm_failed: {error}"),
                );
                job.status = ConsolidationStatus::Pending;
                job.error = Some(error.to_string());
                job.finished_at = Some(Utc::now());
                self.store.save_consolidation_job(&job).await?;
                return Ok(MemoryConsolidationReport {
                    job,
                    message: "LLM 未配置或调用失败，已保留为 pending".to_string(),
                });
            }
        };

        let mut created_or_updated = 0;
        for draft in output.l0 {
            self.memory_store.save_l0(&draft.into_memory()).await?;
            created_or_updated += 1;
        }
        for draft in output.l1_concepts {
            self.memory_store
                .save_l1_concept(&draft.into_memory())
                .await?;
            created_or_updated += 1;
        }
        for draft in output.l1_relations {
            self.memory_store
                .save_l1_relation(&draft.into_memory())
                .await?;
            created_or_updated += 1;
        }
        for draft in output.l2_events {
            self.memory_store.save_l2(&draft.into_memory(date)).await?;
            created_or_updated += 1;
        }
        for draft in output.l3_reflections {
            self.memory_store.save_l3(&draft.into_memory()).await?;
            created_or_updated += 1;
        }

        let mut archived_count = 0;
        for archive in output.archive_ids {
            if self.archive_item(&archive).await? {
                archived_count += 1;
            }
        }
        archived_count += self
            .store
            .archive_stale_reflections(
                settings.memory.decay_after_days,
                settings.memory.archive_after_days,
            )
            .await?;
        self.store
            .cleanup_raw_events(settings.memory.raw_retention_days)
            .await?;
        self.store.clear_working_memory_for_date(date).await?;

        job.status = ConsolidationStatus::Completed;
        job.created_count = created_or_updated;
        job.updated_count = created_or_updated;
        job.archived_count = archived_count;
        job.finished_at = Some(Utc::now());
        self.store.save_consolidation_job(&job).await?;
        debug::log(
            "memory:consolidator:run",
            format!(
                "date={date} completed created_or_updated={created_or_updated} archived={archived_count}"
            ),
        );

        Ok(MemoryConsolidationReport {
            job,
            message: "记忆沉淀完成".to_string(),
        })
    }

    pub async fn run_backfill(&self) -> QPawResult<()> {
        let today = Local::now().date_naive();
        let mut dates = self.store.pending_consolidation_dates().await?;
        dates.extend(
            self.store
                .list_interaction_events()
                .await?
                .into_iter()
                .map(|event| event.created_at.date_naive())
                .filter(|date| *date < today),
        );
        dates.sort();
        dates.dedup();
        debug::log(
            "memory:consolidator:backfill",
            format!("candidate_dates={}", dates.len()),
        );

        for date in dates {
            if self
                .store
                .consolidation_job_for_date(date)
                .await?
                .is_some_and(|job| job.status == ConsolidationStatus::Completed)
            {
                continue;
            }
            debug::log(
                "memory:consolidator:backfill",
                format!("running date={date}"),
            );
            let _ = self.run(date).await?;
        }

        Ok(())
    }

    fn running_job(&self, date: NaiveDate, input_event_count: usize) -> MemoryConsolidationJob {
        MemoryConsolidationJob {
            id: stable_id("memory_job", &date.to_string()),
            date,
            status: ConsolidationStatus::Running,
            input_event_count,
            created_count: 0,
            updated_count: 0,
            archived_count: 0,
            error: None,
            started_at: Utc::now(),
            finished_at: None,
        }
    }

    fn pending_job(
        &self,
        date: NaiveDate,
        input_event_count: usize,
        error: Option<String>,
    ) -> MemoryConsolidationJob {
        MemoryConsolidationJob {
            id: stable_id("memory_job", &date.to_string()),
            date,
            status: ConsolidationStatus::Pending,
            input_event_count,
            created_count: 0,
            updated_count: 0,
            archived_count: 0,
            error,
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
        }
    }

    async fn archive_item(&self, archive: &ArchiveDraft) -> QPawResult<bool> {
        let now = Utc::now();
        match archive.layer.as_str() {
            "l0" => {
                if let Some(mut item) = self
                    .store
                    .list_l0()
                    .await?
                    .into_iter()
                    .find(|i| i.id == archive.id)
                {
                    item.status = MemoryStatus::Archived;
                    item.updated_at = now;
                    self.store.save_l0(&item).await?;
                    return Ok(true);
                }
            }
            "l1_concept" => {
                if let Some(mut item) = self
                    .store
                    .list_l1_concepts()
                    .await?
                    .into_iter()
                    .find(|i| i.id == archive.id)
                {
                    item.status = MemoryStatus::Archived;
                    item.updated_at = now;
                    self.store.save_l1_concept(&item).await?;
                    return Ok(true);
                }
            }
            "l1_relation" => {
                if let Some(mut item) = self
                    .store
                    .list_l1_relations()
                    .await?
                    .into_iter()
                    .find(|i| i.id == archive.id)
                {
                    item.status = MemoryStatus::Archived;
                    item.updated_at = now;
                    self.store.save_l1_relation(&item).await?;
                    return Ok(true);
                }
            }
            "l2" => {
                if let Some(mut item) = self
                    .store
                    .list_l2()
                    .await?
                    .into_iter()
                    .find(|i| i.id == archive.id)
                {
                    item.status = MemoryStatus::Archived;
                    item.updated_at = now;
                    self.store.save_l2(&item).await?;
                    return Ok(true);
                }
            }
            "l3" => {
                if let Some(mut item) = self
                    .store
                    .list_l3()
                    .await?
                    .into_iter()
                    .find(|i| i.id == archive.id)
                {
                    item.status = MemoryStatus::Archived;
                    item.updated_at = now;
                    self.store.save_l3(&item).await?;
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }
}

impl L0Draft {
    fn into_memory(self) -> MemoryL0 {
        let now = Utc::now();
        let title = clean(self.title);
        MemoryL0 {
            id: stable_id("l0", &format!("{}:{title}", self.category)),
            category: parse_category(&self.category),
            title,
            summary: clean(self.summary),
            tags: clean_tags(self.tags),
            confidence: clamp01(self.confidence),
            evidence_event_ids: dedupe(self.evidence_event_ids),
            status: MemoryStatus::Active,
            created_at: now,
            updated_at: now,
            last_touched_at: now,
        }
    }
}

impl L1ConceptDraft {
    fn into_memory(self) -> MemoryL1Concept {
        let now = Utc::now();
        let name = clean(self.name);
        MemoryL1Concept {
            id: stable_id("l1_concept", &format!("{}:{name}", self.concept_type)),
            name,
            concept_type: clean(self.concept_type),
            aliases: dedupe(self.aliases.into_iter().map(clean).collect()),
            summary: clean(self.summary),
            tags: clean_tags(self.tags),
            confidence: clamp01(self.confidence),
            evidence_event_ids: dedupe(self.evidence_event_ids),
            status: MemoryStatus::Active,
            created_at: now,
            updated_at: now,
            last_touched_at: now,
        }
    }
}

impl L1RelationDraft {
    fn into_memory(self) -> MemoryL1Relation {
        let now = Utc::now();
        let subject = clean(self.subject);
        let predicate = clean(self.predicate);
        let object = clean(self.object);
        MemoryL1Relation {
            id: stable_id("l1_relation", &format!("{subject}:{predicate}:{object}")),
            subject,
            predicate,
            object,
            summary: clean(self.summary),
            tags: clean_tags(self.tags),
            confidence: clamp01(self.confidence),
            evidence_event_ids: dedupe(self.evidence_event_ids),
            status: MemoryStatus::Active,
            created_at: now,
            updated_at: now,
            last_touched_at: now,
        }
    }
}

impl L2EventDraft {
    fn into_memory(self, date: NaiveDate) -> MemoryL2Event {
        let now = Utc::now();
        let title = clean(self.title);
        MemoryL2Event {
            id: stable_id("l2", &format!("{date}:{title}")),
            title,
            summary: clean(self.summary),
            occurred_at: now,
            entity_ids: dedupe(self.entity_ids.into_iter().map(clean).collect()),
            tags: clean_tags(self.tags),
            importance: clamp01(self.importance),
            source_event_ids: dedupe(self.source_event_ids),
            status: MemoryStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }
}

impl L3ReflectionDraft {
    fn into_memory(self) -> MemoryL3Reflection {
        let now = Utc::now();
        let title = clean(self.title);
        MemoryL3Reflection {
            id: stable_id("l3", &title),
            kind: parse_reflection_kind(&self.kind),
            title,
            insight: clean(self.insight),
            application: clean(self.application),
            tags: clean_tags(self.tags),
            confidence: clamp01(self.confidence),
            evidence_event_ids: dedupe(self.evidence_event_ids),
            status: MemoryStatus::Active,
            created_at: now,
            updated_at: now,
            last_touched_at: now,
        }
    }
}

fn parse_output(content: &str) -> QPawResult<ConsolidationOutput> {
    let start = content
        .find('{')
        .ok_or_else(|| QPawError::Message("missing JSON object start".to_string()))?;
    let end = content
        .rfind('}')
        .ok_or_else(|| QPawError::Message("missing JSON object end".to_string()))?;
    Ok(serde_json::from_str(&content[start..=end])?)
}

fn parse_category(value: &str) -> MemoryL0Category {
    match value {
        "person_relation" => MemoryL0Category::PersonRelation,
        "task_project" => MemoryL0Category::TaskProject,
        "health_habit" => MemoryL0Category::HealthHabit,
        "interaction_style" => MemoryL0Category::InteractionStyle,
        "lesson" => MemoryL0Category::Lesson,
        _ => MemoryL0Category::Preference,
    }
}

fn parse_reflection_kind(value: &str) -> ReflectionKind {
    match value {
        "success" => ReflectionKind::Success,
        "failure" => ReflectionKind::Failure,
        _ => ReflectionKind::Observation,
    }
}

fn stable_id(prefix: &str, key: &str) -> String {
    let mut hasher = StableHasher::default();
    key.to_lowercase().trim().hash(&mut hasher);
    format!("{prefix}_{:016x}", hasher.finish())
}

#[derive(Default)]
struct StableHasher(u64);

impl Hasher for StableHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

fn clean(value: String) -> String {
    value.trim().chars().take(600).collect()
}

fn clean_tags(tags: Vec<String>) -> Vec<String> {
    dedupe(
        tags.into_iter()
            .map(clean)
            .filter(|tag| !tag.is_empty())
            .take(12)
            .collect(),
    )
}

fn dedupe(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.to_lowercase()) {
            out.push(value);
        }
    }
    out
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_inside_markdown_fence() {
        let output = parse_output("```json\n{\"l0\":[]}\n```").unwrap();
        assert!(output.l0.is_empty());
    }

    #[test]
    fn stable_ids_are_repeatable() {
        assert_eq!(stable_id("l0", "Water"), stable_id("l0", " water "));
    }

    #[test]
    fn unknown_category_defaults_to_preference() {
        assert_eq!(parse_category("unknown"), MemoryL0Category::Preference);
    }
}
