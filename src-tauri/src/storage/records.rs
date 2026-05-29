use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::models::{
    AvatarManifest, ConsolidationStatus, ExplicitMemoryItem, ExplicitMemoryStatus,
    InteractionEvent, InteractionEventKind, LayeredMemoryItem, MemoryConsolidationJob, MemoryL0,
    MemoryL0Category, MemoryL1Concept, MemoryL1Relation, MemoryL2Event, MemoryL3Reflection,
    MemoryLayer, MemoryStatus, ReflectionKind, WorkingMemoryItem, WorkingMemoryKind,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SchemaMigrationRecord {
    pub(super) applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AvatarManifestRecord {
    pub(super) uid: String,
    pub(super) name: String,
    pub(super) kind: crate::models::AvatarKind,
    pub(super) path: String,
    pub(super) model_json_path: Option<String>,
    pub(super) image_path: Option<String>,
    pub(super) imported_at: DateTime<Utc>,
}

impl From<&AvatarManifest> for AvatarManifestRecord {
    fn from(item: &AvatarManifest) -> Self {
        Self {
            uid: item.id.clone(),
            name: item.name.clone(),
            kind: item.kind.clone(),
            path: item.path.clone(),
            model_json_path: item.model_json_path.clone(),
            image_path: item.image_path.clone(),
            imported_at: item.imported_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct InteractionEventRecord {
    pub(super) uid: String,
    pub(super) kind: InteractionEventKind,
    pub(super) actor: String,
    pub(super) summary: String,
    pub(super) content_json: String,
    pub(super) tags: Vec<String>,
    pub(super) created_at: DateTime<Utc>,
}

impl TryFrom<&InteractionEvent> for InteractionEventRecord {
    type Error = serde_json::Error;

    fn try_from(item: &InteractionEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            uid: item.id.clone(),
            kind: item.kind.clone(),
            actor: item.actor.clone(),
            summary: item.summary.clone(),
            content_json: serde_json::to_string(&item.content)?,
            tags: item.tags.clone(),
            created_at: item.created_at,
        })
    }
}

impl TryFrom<InteractionEventRecord> for InteractionEvent {
    type Error = serde_json::Error;

    fn try_from(item: InteractionEventRecord) -> Result<Self, Self::Error> {
        Ok(Self {
            id: item.uid,
            kind: item.kind,
            actor: item.actor,
            summary: item.summary,
            content: serde_json::from_str(&item.content_json)?,
            tags: item.tags,
            created_at: item.created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkingMemoryRecord {
    pub(super) uid: String,
    pub(super) kind: WorkingMemoryKind,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) keywords: Vec<String>,
    pub(super) source_event_ids: Vec<String>,
    pub(super) confidence: f64,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) expires_at: DateTime<Utc>,
}

impl From<&WorkingMemoryItem> for WorkingMemoryRecord {
    fn from(item: &WorkingMemoryItem) -> Self {
        Self {
            uid: item.id.clone(),
            kind: item.kind.clone(),
            title: item.title.clone(),
            summary: item.summary.clone(),
            keywords: item.keywords.clone(),
            source_event_ids: item.source_event_ids.clone(),
            confidence: item.confidence,
            created_at: item.created_at,
            updated_at: item.updated_at,
            expires_at: item.expires_at,
        }
    }
}

impl From<WorkingMemoryRecord> for WorkingMemoryItem {
    fn from(item: WorkingMemoryRecord) -> Self {
        Self {
            id: item.uid,
            kind: item.kind,
            title: item.title,
            summary: item.summary,
            keywords: item.keywords,
            source_event_ids: item.source_event_ids,
            confidence: item.confidence,
            created_at: item.created_at,
            updated_at: item.updated_at,
            expires_at: item.expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ExplicitMemoryRecord {
    pub(super) uid: String,
    pub(super) normalized_body: String,
    pub(super) body: String,
    pub(super) source: String,
    pub(super) tags: Vec<String>,
    pub(super) keywords: Vec<String>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) last_used_at: DateTime<Utc>,
    pub(super) status: ExplicitMemoryStatus,
}

impl ExplicitMemoryRecord {
    pub(super) fn from_item(item: &ExplicitMemoryItem, normalized_body: String) -> Self {
        Self {
            uid: item.id.clone(),
            normalized_body,
            body: item.body.clone(),
            source: item.source.clone(),
            tags: item.tags.clone(),
            keywords: item.keywords.clone(),
            created_at: item.created_at,
            last_used_at: item.last_used_at,
            status: item.status.clone(),
        }
    }
}

impl From<ExplicitMemoryRecord> for ExplicitMemoryItem {
    fn from(item: ExplicitMemoryRecord) -> Self {
        Self {
            id: item.uid,
            body: item.body,
            source: item.source,
            tags: item.tags,
            keywords: item.keywords,
            created_at: item.created_at,
            last_used_at: item.last_used_at,
            status: item.status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryL0Record {
    pub(super) uid: String,
    pub(super) category: MemoryL0Category,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) tags: Vec<String>,
    pub(super) confidence: f64,
    pub(super) evidence_event_ids: Vec<String>,
    pub(super) status: MemoryStatus,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) last_touched_at: DateTime<Utc>,
}

impl From<&MemoryL0> for MemoryL0Record {
    fn from(item: &MemoryL0) -> Self {
        Self {
            uid: item.id.clone(),
            category: item.category.clone(),
            title: item.title.clone(),
            summary: item.summary.clone(),
            tags: item.tags.clone(),
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids.clone(),
            status: item.status.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

impl From<MemoryL0Record> for MemoryL0 {
    fn from(item: MemoryL0Record) -> Self {
        Self {
            id: item.uid,
            category: item.category,
            title: item.title,
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryL1ConceptRecord {
    pub(super) uid: String,
    pub(super) name: String,
    pub(super) concept_type: String,
    pub(super) aliases: Vec<String>,
    pub(super) summary: String,
    pub(super) tags: Vec<String>,
    pub(super) confidence: f64,
    pub(super) evidence_event_ids: Vec<String>,
    pub(super) status: MemoryStatus,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) last_touched_at: DateTime<Utc>,
}

impl From<&MemoryL1Concept> for MemoryL1ConceptRecord {
    fn from(item: &MemoryL1Concept) -> Self {
        Self {
            uid: item.id.clone(),
            name: item.name.clone(),
            concept_type: item.concept_type.clone(),
            aliases: item.aliases.clone(),
            summary: item.summary.clone(),
            tags: item.tags.clone(),
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids.clone(),
            status: item.status.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

impl From<MemoryL1ConceptRecord> for MemoryL1Concept {
    fn from(item: MemoryL1ConceptRecord) -> Self {
        Self {
            id: item.uid,
            name: item.name,
            concept_type: item.concept_type,
            aliases: item.aliases,
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryL1RelationRecord {
    pub(super) uid: String,
    pub(super) subject: String,
    pub(super) predicate: String,
    pub(super) object: String,
    pub(super) summary: String,
    pub(super) tags: Vec<String>,
    pub(super) confidence: f64,
    pub(super) evidence_event_ids: Vec<String>,
    pub(super) status: MemoryStatus,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) last_touched_at: DateTime<Utc>,
}

impl From<&MemoryL1Relation> for MemoryL1RelationRecord {
    fn from(item: &MemoryL1Relation) -> Self {
        Self {
            uid: item.id.clone(),
            subject: item.subject.clone(),
            predicate: item.predicate.clone(),
            object: item.object.clone(),
            summary: item.summary.clone(),
            tags: item.tags.clone(),
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids.clone(),
            status: item.status.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

impl From<MemoryL1RelationRecord> for MemoryL1Relation {
    fn from(item: MemoryL1RelationRecord) -> Self {
        Self {
            id: item.uid,
            subject: item.subject,
            predicate: item.predicate,
            object: item.object,
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryL2EventRecord {
    pub(super) uid: String,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) occurred_at: DateTime<Utc>,
    pub(super) entity_ids: Vec<String>,
    pub(super) tags: Vec<String>,
    pub(super) importance: f64,
    pub(super) source_event_ids: Vec<String>,
    pub(super) status: MemoryStatus,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
}

impl From<&MemoryL2Event> for MemoryL2EventRecord {
    fn from(item: &MemoryL2Event) -> Self {
        Self {
            uid: item.id.clone(),
            title: item.title.clone(),
            summary: item.summary.clone(),
            occurred_at: item.occurred_at,
            entity_ids: item.entity_ids.clone(),
            tags: item.tags.clone(),
            importance: item.importance,
            source_event_ids: item.source_event_ids.clone(),
            status: item.status.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}

impl From<MemoryL2EventRecord> for MemoryL2Event {
    fn from(item: MemoryL2EventRecord) -> Self {
        Self {
            id: item.uid,
            title: item.title,
            summary: item.summary,
            occurred_at: item.occurred_at,
            entity_ids: item.entity_ids,
            tags: item.tags,
            importance: item.importance,
            source_event_ids: item.source_event_ids,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryL3ReflectionRecord {
    pub(super) uid: String,
    pub(super) kind: ReflectionKind,
    pub(super) title: String,
    pub(super) insight: String,
    pub(super) application: String,
    pub(super) tags: Vec<String>,
    pub(super) confidence: f64,
    pub(super) evidence_event_ids: Vec<String>,
    pub(super) status: MemoryStatus,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
    pub(super) last_touched_at: DateTime<Utc>,
}

impl From<&MemoryL3Reflection> for MemoryL3ReflectionRecord {
    fn from(item: &MemoryL3Reflection) -> Self {
        Self {
            uid: item.id.clone(),
            kind: item.kind.clone(),
            title: item.title.clone(),
            insight: item.insight.clone(),
            application: item.application.clone(),
            tags: item.tags.clone(),
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids.clone(),
            status: item.status.clone(),
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

impl From<MemoryL3ReflectionRecord> for MemoryL3Reflection {
    fn from(item: MemoryL3ReflectionRecord) -> Self {
        Self {
            id: item.uid,
            kind: item.kind,
            title: item.title,
            insight: item.insight,
            application: item.application,
            tags: item.tags,
            confidence: item.confidence,
            evidence_event_ids: item.evidence_event_ids,
            status: item.status,
            created_at: item.created_at,
            updated_at: item.updated_at,
            last_touched_at: item.last_touched_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MemoryConsolidationJobRecord {
    pub(super) uid: String,
    pub(super) date: NaiveDate,
    pub(super) status: ConsolidationStatus,
    pub(super) input_event_count: usize,
    pub(super) created_count: usize,
    pub(super) updated_count: usize,
    pub(super) archived_count: usize,
    pub(super) error: Option<String>,
    pub(super) started_at: DateTime<Utc>,
    pub(super) finished_at: Option<DateTime<Utc>>,
}

impl From<&MemoryConsolidationJob> for MemoryConsolidationJobRecord {
    fn from(item: &MemoryConsolidationJob) -> Self {
        Self {
            uid: item.id.clone(),
            date: item.date,
            status: item.status.clone(),
            input_event_count: item.input_event_count,
            created_count: item.created_count,
            updated_count: item.updated_count,
            archived_count: item.archived_count,
            error: item.error.clone(),
            started_at: item.started_at,
            finished_at: item.finished_at,
        }
    }
}

impl From<MemoryConsolidationJobRecord> for MemoryConsolidationJob {
    fn from(item: MemoryConsolidationJobRecord) -> Self {
        Self {
            id: item.uid,
            date: item.date,
            status: item.status,
            input_event_count: item.input_event_count,
            created_count: item.created_count,
            updated_count: item.updated_count,
            archived_count: item.archived_count,
            error: item.error,
            started_at: item.started_at,
            finished_at: item.finished_at,
        }
    }
}

impl From<MemoryL0> for LayeredMemoryItem {
    fn from(item: MemoryL0) -> Self {
        Self {
            id: item.id,
            layer: MemoryLayer::L0,
            category: Some(item.category),
            title: item.title,
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            status: item.status,
            updated_at: item.updated_at,
        }
    }
}

impl From<MemoryL1Concept> for LayeredMemoryItem {
    fn from(item: MemoryL1Concept) -> Self {
        Self {
            id: item.id,
            layer: MemoryLayer::L1Concept,
            category: None,
            title: item.name,
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            status: item.status,
            updated_at: item.updated_at,
        }
    }
}

impl From<MemoryL1Relation> for LayeredMemoryItem {
    fn from(item: MemoryL1Relation) -> Self {
        Self {
            id: item.id,
            layer: MemoryLayer::L1Relation,
            category: None,
            title: format!("{} {} {}", item.subject, item.predicate, item.object),
            summary: item.summary,
            tags: item.tags,
            confidence: item.confidence,
            status: item.status,
            updated_at: item.updated_at,
        }
    }
}

impl From<MemoryL2Event> for LayeredMemoryItem {
    fn from(item: MemoryL2Event) -> Self {
        Self {
            id: item.id,
            layer: MemoryLayer::L2,
            category: None,
            title: item.title,
            summary: item.summary,
            tags: item.tags,
            confidence: item.importance,
            status: item.status,
            updated_at: item.updated_at,
        }
    }
}

impl From<MemoryL3Reflection> for LayeredMemoryItem {
    fn from(item: MemoryL3Reflection) -> Self {
        Self {
            id: item.id,
            layer: MemoryLayer::L3,
            category: Some(crate::models::MemoryL0Category::Lesson),
            title: item.title,
            summary: item.insight,
            tags: item.tags,
            confidence: item.confidence,
            status: item.status,
            updated_at: item.updated_at,
        }
    }
}
