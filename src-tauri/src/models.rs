use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4.1-mini".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderSettings {
    pub paused: bool,
    #[serde(default = "default_reminder_items")]
    pub items: Vec<ReminderDefinition>,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            paused: false,
            items: default_reminder_items(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderDefinition {
    pub id: String,
    pub title: String,
    pub message: String,
    pub action_label: String,
    pub interval_minutes: u64,
    pub idle_grace_minutes: u64,
    pub paused: bool,
}

fn default_reminder_items() -> Vec<ReminderDefinition> {
    vec![
        ReminderDefinition {
            id: "eye_rest".to_string(),
            title: "闭眼休息".to_string(),
            message: "眼睛也陪你努力很久了。闭上眼休息一小会儿，把清亮还给自己。".to_string(),
            action_label: "照顾好了".to_string(),
            interval_minutes: 45,
            idle_grace_minutes: 10,
            paused: false,
        },
        ReminderDefinition {
            id: "hydration".to_string(),
            title: "喝水时间".to_string(),
            message: "该喝点水了。慢慢喝几口，让身体从安静的地方重新亮起来。".to_string(),
            action_label: "照顾好了".to_string(),
            interval_minutes: 60,
            idle_grace_minutes: 10,
            paused: false,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvatarKind {
    Live2d,
    Image,
    BuiltIn,
}

fn default_avatar_kind() -> AvatarKind {
    AvatarKind::BuiltIn
}

fn default_avatar_id() -> Option<String> {
    Some("star_lantern_cat".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarSettings {
    #[serde(default = "default_avatar_id")]
    pub current_avatar_id: Option<String>,
    #[serde(default = "default_avatar_kind")]
    pub kind: AvatarKind,
    #[serde(default)]
    pub model_json_path: Option<String>,
    #[serde(default)]
    pub image_path: Option<String>,
    #[serde(default = "default_avatar_scale")]
    pub scale: f64,
}

fn default_avatar_scale() -> f64 {
    1.0
}

impl Default for AvatarSettings {
    fn default() -> Self {
        Self {
            current_avatar_id: default_avatar_id(),
            kind: default_avatar_kind(),
            model_json_path: None,
            image_path: None,
            scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub pet_x: Option<i32>,
    pub pet_y: Option<i32>,
    pub pet_scale: f64,
    pub pet_width: Option<u32>,
    pub pet_height: Option<u32>,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            pet_x: None,
            pet_y: None,
            pet_scale: 1.0,
            pet_width: None,
            pet_height: None,
        }
    }
}

pub const PET_WINDOW_MIN_WIDTH: u32 = 80;
pub const PET_WINDOW_MIN_HEIGHT: u32 = 100;
pub const PET_WINDOW_MAX_WIDTH: u32 = 2000;
pub const PET_WINDOW_MAX_HEIGHT: u32 = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySettings {
    pub enabled: bool,
    pub raw_retention_days: u64,
    pub decay_after_days: u64,
    pub archive_after_days: u64,
    pub consolidation_time: String,
    #[serde(default = "default_working_memory_enabled")]
    pub working_memory_enabled: bool,
    #[serde(default = "default_working_memory_retention_hours")]
    pub working_memory_retention_hours: u64,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            raw_retention_days: 30,
            decay_after_days: 90,
            archive_after_days: 180,
            consolidation_time: "00:00".to_string(),
            working_memory_enabled: default_working_memory_enabled(),
            working_memory_retention_hours: default_working_memory_retention_hours(),
        }
    }
}

fn default_working_memory_enabled() -> bool {
    true
}

fn default_working_memory_retention_hours() -> u64 {
    36
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub llm: LlmConfig,
    pub reminders: ReminderSettings,
    pub avatar: AvatarSettings,
    pub window: WindowSettings,
    #[serde(default)]
    pub memory: MemorySettings,
    pub privacy_scope: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            reminders: ReminderSettings::default(),
            avatar: AvatarSettings::default(),
            window: WindowSettings::default(),
            memory: MemorySettings::default(),
            privacy_scope: "minimal".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarManifest {
    pub id: String,
    pub name: String,
    pub kind: AvatarKind,
    pub path: String,
    pub model_json_path: Option<String>,
    pub image_path: Option<String>,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDocument {
    pub body: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionEventKind {
    ChatMessage,
    ReminderFeedback,
    TaskEvent,
    AssistantReflection,
    AppAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionEvent {
    pub id: String,
    pub kind: InteractionEventKind,
    pub actor: String,
    pub summary: String,
    pub content: Value,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkingMemoryKind {
    Identity,
    Preference,
    TaskProject,
    HealthHabit,
    InteractionStyle,
    RecentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryItem {
    pub id: String,
    pub kind: WorkingMemoryKind,
    pub title: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub source_event_ids: Vec<String>,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
    L0,
    L1Concept,
    L1Relation,
    L2,
    L3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryL0Category {
    Preference,
    PersonRelation,
    TaskProject,
    HealthHabit,
    InteractionStyle,
    Lesson,
}

impl MemoryL0Category {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Preference,
            Self::PersonRelation,
            Self::TaskProject,
            Self::HealthHabit,
            Self::InteractionStyle,
            Self::Lesson,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    Decayed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryL0 {
    pub id: String,
    pub category: MemoryL0Category,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub evidence_event_ids: Vec<String>,
    pub status: MemoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_touched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryL1Concept {
    pub id: String,
    pub name: String,
    pub concept_type: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub evidence_event_ids: Vec<String>,
    pub status: MemoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_touched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryL1Relation {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub evidence_event_ids: Vec<String>,
    pub status: MemoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_touched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryL2Event {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub occurred_at: DateTime<Utc>,
    pub entity_ids: Vec<String>,
    pub tags: Vec<String>,
    pub importance: f64,
    pub source_event_ids: Vec<String>,
    pub status: MemoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReflectionKind {
    Success,
    Failure,
    Observation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryL3Reflection {
    pub id: String,
    pub kind: ReflectionKind,
    pub title: String,
    pub insight: String,
    pub application: String,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub evidence_event_ids: Vec<String>,
    pub status: MemoryStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_touched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationJob {
    pub id: String,
    pub date: NaiveDate,
    pub status: ConsolidationStatus,
    pub input_event_count: usize,
    pub created_count: usize,
    pub updated_count: usize,
    pub archived_count: usize,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryLayerFilter {
    pub layer: Option<MemoryLayer>,
    pub category: Option<MemoryL0Category>,
    pub query: Option<String>,
    pub include_archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredMemoryItem {
    pub id: String,
    pub layer: MemoryLayer,
    pub category: Option<MemoryL0Category>,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub confidence: f64,
    pub status: MemoryStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryQueryRequest {
    pub query: String,
    pub layer: Option<MemoryLayer>,
    pub category: Option<MemoryL0Category>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResponse {
    pub items: Vec<LayeredMemoryItem>,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsolidationReport {
    pub job: MemoryConsolidationJob,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStats {
    pub raw_events: usize,
    pub l0: usize,
    pub l1_concepts: usize,
    pub l1_relations: usize,
    pub l2_events: usize,
    pub l3_reflections: usize,
    pub pending_jobs: usize,
    pub last_consolidation_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitEvent {
    pub active: bool,
    pub idle_seconds: u64,
    pub created_at: DateTime<Utc>,
}

pub type ReminderKind = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderPayload {
    pub id: String,
    pub kind: ReminderKind,
    pub title: String,
    pub message: String,
    pub action_label: String,
    pub due_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderRuntimeStatus {
    pub paused: bool,
    pub idle_seconds: u64,
    pub active: bool,
    pub items: Vec<ReminderItemRuntimeStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderItemRuntimeStatus {
    pub id: String,
    pub title: String,
    pub paused: bool,
    pub active_seconds: u64,
    pub interval_seconds: u64,
    pub idle_grace_seconds: u64,
    pub pending_waited_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderFeedback {
    Done,
    Snoozed,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderFeedbackPayload {
    pub reminder_id: String,
    pub kind: ReminderKind,
    pub feedback: ReminderFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderEvent {
    pub reminder_id: String,
    pub kind: ReminderKind,
    pub message: String,
    pub feedback: Option<ReminderFeedback>,
    pub idle_seconds: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendChatResponse {
    pub user: ChatMessage,
    pub assistant: ChatMessage,
    pub memories: Vec<MemoryDocument>,
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, AvatarKind};

    #[test]
    fn default_avatar_uses_built_in_star_lantern_cat() {
        let settings = AppSettings::default();

        assert_eq!(settings.avatar.kind, AvatarKind::BuiltIn);
        assert_eq!(
            settings.avatar.current_avatar_id.as_deref(),
            Some("star_lantern_cat")
        );
        assert!(settings.avatar.image_path.is_none());
        assert!(settings.avatar.model_json_path.is_none());
    }

    #[test]
    fn avatar_kind_serializes_as_frontend_snake_case() {
        let encoded = serde_json::to_value(AvatarKind::BuiltIn).expect("serialize avatar kind");

        assert_eq!(encoded, serde_json::json!("built_in"));
    }
}
