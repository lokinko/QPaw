export type ReminderKind = string;

export const DEFAULT_AVATAR_IMAGE_PATH = "/concepts/star-lantern-cat-concept.png";
export const DEFAULT_BUILT_IN_AVATAR_ID = "star_lantern_cat";

export type ReminderFeedback = "done" | "snoozed" | "dismissed";

export interface LlmConfig {
  base_url: string;
  api_key: string;
  model: string;
}

export interface CodexDevStatus {
  cli_installed: boolean;
  cli_version: string | null;
  auth_file_found: boolean;
  auth_file_path: string | null;
  app_server_available: boolean;
}

export interface ReminderSettings {
  paused: boolean;
  items: ReminderDefinition[];
}

export interface ReminderDefinition {
  id: string;
  title: string;
  message: string;
  action_label: string;
  interval_minutes: number;
  idle_grace_minutes: number;
  paused: boolean;
}

export interface AvatarSettings {
  current_avatar_id: string | null;
  kind: AvatarKind;
  model_json_path: string | null;
  image_path: string | null;
  scale: number;
}

export type AvatarKind = "live2d" | "image" | "built_in";

export interface WindowSettings {
  pet_x: number | null;
  pet_y: number | null;
  pet_scale: number;
  pet_width: number | null;
  pet_height: number | null;
}

export interface MemorySettings {
  enabled: boolean;
  raw_retention_days: number;
  decay_after_days: number;
  archive_after_days: number;
  consolidation_time: string;
  working_memory_enabled: boolean;
  working_memory_retention_hours: number;
}

export interface AppSettings {
  llm: LlmConfig;
  reminders: ReminderSettings;
  avatar: AvatarSettings;
  window: WindowSettings;
  memory: MemorySettings;
  privacy_scope: "minimal";
}

export interface AvatarManifest {
  id: string;
  name: string;
  kind: AvatarKind;
  path: string;
  model_json_path: string | null;
  image_path: string | null;
  imported_at: string;
}

export interface ChatMessage {
  id?: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
}

export interface MemoryDocument {
  id?: string;
  body: string;
  source: string;
  created_at: string;
}

export interface ReminderPayload {
  id: string;
  kind: ReminderKind;
  title: string;
  message: string;
  action_label: string;
  due_at: string;
}

export interface ReminderRuntimeStatus {
  paused: boolean;
  idle_seconds: number;
  active: boolean;
  items: ReminderItemRuntimeStatus[];
}

export interface ReminderItemRuntimeStatus {
  id: string;
  title: string;
  paused: boolean;
  active_seconds: number;
  interval_seconds: number;
  idle_grace_seconds: number;
  pending_waited_seconds: number | null;
}

export interface ReminderFeedbackPayload {
  reminder_id: string;
  kind: ReminderKind;
  feedback: ReminderFeedback;
}

export interface SendChatResponse {
  user: ChatMessage;
  assistant: ChatMessage;
  memories: MemoryDocument[];
}

export type WorkingMemoryKind =
  | "identity"
  | "preference"
  | "task_project"
  | "health_habit"
  | "interaction_style"
  | "recent_context";

export interface WorkingMemoryItem {
  id: string;
  kind: WorkingMemoryKind;
  title: string;
  summary: string;
  keywords: string[];
  source_event_ids: string[];
  confidence: number;
  created_at: string;
  updated_at: string;
  expires_at: string;
}

export type MemoryLayer = "l0" | "l1_concept" | "l1_relation" | "l2" | "l3";

export type MemoryL0Category =
  | "preference"
  | "person_relation"
  | "task_project"
  | "health_habit"
  | "interaction_style"
  | "lesson";

export type MemoryStatus = "active" | "decayed" | "archived";

export interface MemoryLayerFilter {
  layer?: MemoryLayer | null;
  category?: MemoryL0Category | null;
  query?: string | null;
  include_archived: boolean;
}

export interface LayeredMemoryItem {
  id: string;
  layer: MemoryLayer;
  category: MemoryL0Category | null;
  title: string;
  summary: string;
  tags: string[];
  confidence: number;
  status: MemoryStatus;
  updated_at: string;
}

export interface MemoryQueryRequest {
  query: string;
  layer?: MemoryLayer | null;
  category?: MemoryL0Category | null;
  limit?: number | null;
}

export interface MemoryQueryResponse {
  items: LayeredMemoryItem[];
  context: string;
}

export interface MemoryConsolidationJob {
  id: string;
  date: string;
  status: "pending" | "running" | "completed" | "failed";
  input_event_count: number;
  created_count: number;
  updated_count: number;
  archived_count: number;
  error: string | null;
  started_at: string;
  finished_at: string | null;
}

export interface MemoryConsolidationReport {
  job: MemoryConsolidationJob;
  message: string;
}

export interface MemoryStats {
  raw_events: number;
  l0: number;
  l1_concepts: number;
  l1_relations: number;
  l2_events: number;
  l3_reflections: number;
  pending_jobs: number;
  last_consolidation_at: string | null;
}
