import type {
  AppSettings,
  AvatarManifest,
  ChatMessage,
  CodexDevStatus,
  LayeredMemoryItem,
  LlmConnectionTestResult,
  MemoryDocument,
  MemoryConsolidationReport,
  MemoryLayerFilter,
  MemoryStats,
  ReminderFeedbackPayload,
  ReminderPayload,
  ReminderRuntimeStatus,
  SendChatResponse,
  WorkingMemoryItem,
} from "./types";
import { DEFAULT_BUILT_IN_AVATAR_ID } from "./types";

const now = () => new Date().toISOString();

export const fallbackSettings: AppSettings = {
  llm: {
    provider: "codex_cli",
    base_url: "https://api.openai.com/v1",
    api_key: "",
    model: "",
    provider_configs: {
      codex_cli: {
        base_url: "",
        api_key: "",
        model: "",
      },
      open_ai_compatible: {
        base_url: "https://api.openai.com/v1",
        api_key: "",
        model: "gpt-4.1-mini",
      },
    },
  },
  reminders: {
    paused: false,
    items: [
      {
        id: "eye_rest",
        title: "闭眼休息",
        message: "眼睛也陪你努力很久了。闭上眼休息一小会儿，把清亮还给自己。",
        action_label: "照顾好了",
        interval_minutes: 45,
        idle_grace_minutes: 10,
        paused: false,
      },
      {
        id: "hydration",
        title: "喝水时间",
        message: "该喝点水了。慢慢喝几口，让身体从安静的地方重新亮起来。",
        action_label: "照顾好了",
        interval_minutes: 60,
        idle_grace_minutes: 10,
        paused: false,
      },
    ],
  },
  avatar: {
    current_avatar_id: DEFAULT_BUILT_IN_AVATAR_ID,
    kind: "built_in",
    model_json_path: null,
    image_path: null,
    scale: 1,
  },
  window: {
    pet_x: null,
    pet_y: null,
    pet_scale: 1,
    pet_width: null,
    pet_height: null,
  },
  memory: {
    enabled: true,
    raw_retention_days: 30,
    decay_after_days: 90,
    archive_after_days: 180,
    consolidation_time: "00:00",
    working_memory_enabled: true,
    working_memory_retention_hours: 36,
  },
  personal_memory: {
    enabled: true,
    daily_prompt_limit: 2,
    allowed_windows: [
      { start: "13:30", end: "16:30" },
      { start: "20:00", end: "23:00" },
    ],
    idle_threshold_seconds: 180,
    fullscreen_behavior: "hide",
    memory_sensitivity: "balanced",
    allow_confirmation_questions: true,
    allow_low_confidence_in_review: false,
  },
  privacy_scope: "minimal",
};

let settings = structuredClone(fallbackSettings);
let memories: MemoryDocument[] = [];
let chatHistory: ChatMessage[] = [];
let layeredMemories: LayeredMemoryItem[] = [];
let workingMemory: WorkingMemoryItem[] = [];
let fallbackStartedAt = Date.now();

export async function invokeFallback<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  switch (command) {
    case "get_settings":
      return structuredClone(settings) as T;
    case "get_codex_dev_status":
      return {
        cli_installed: false,
        cli_version: null,
        auth_file_found: false,
        auth_file_path: null,
        app_server_available: false,
      } satisfies CodexDevStatus as T;
    case "test_llm_connection": {
      const testSettings = (args?.settings as AppSettings | undefined) ?? settings;
      const missingOpenAiConfig =
        testSettings.llm.provider === "open_ai_compatible" &&
        (!testSettings.llm.base_url || !testSettings.llm.model || !testSettings.llm.api_key);
      return {
        provider: testSettings.llm.provider,
        success: !missingOpenAiConfig,
        message: missingOpenAiConfig
          ? "LLM connectivity test failed"
          : "LLM connectivity test succeeded",
        detail: missingOpenAiConfig ? "OpenAI-compatible provider requires configuration" : "Fallback runtime",
      } satisfies LlmConnectionTestResult as T;
    }
    case "save_settings":
      settings = structuredClone(args?.settings as AppSettings);
      return settings as T;
    case "save_pet_window_size":
      settings.window = {
        ...settings.window,
        pet_width: typeof args?.width === "number" ? args.width : settings.window.pet_width,
        pet_height: typeof args?.height === "number" ? args.height : settings.window.pet_height,
      };
      return undefined as T;
    case "import_avatar": {
      const path = String(args?.path ?? "");
      const kind = isImagePath(path) ? "image" : "live2d";
      const manifest: AvatarManifest = {
        id: crypto.randomUUID(),
        name: avatarName(path),
        kind,
        path,
        model_json_path: kind === "live2d" ? path : null,
        image_path: kind === "image" ? path : null,
        imported_at: now(),
      };
      settings.avatar = {
        current_avatar_id: manifest.id,
        kind: manifest.kind,
        model_json_path: manifest.model_json_path,
        image_path: manifest.image_path,
        scale: settings.avatar.scale,
      };
      return manifest as T;
    }
    case "send_chat_message": {
      const content = String(args?.message ?? "");
      const user = { role: "user" as const, content, created_at: now() };
      const extractedName = extractName(content);
      if (extractedName && settings.memory.working_memory_enabled) {
        const timestamp = now();
        workingMemory = [
          {
            id: "working_identity_qpaw_name",
            kind: "identity",
            title: "QPaw name",
            summary: `QPaw should identify itself as ${extractedName}.`,
            keywords: ["qpaw", "name", extractedName],
            source_event_ids: ["fallback"],
            confidence: 0.95,
            created_at: workingMemory.find((item) => item.id === "working_identity_qpaw_name")?.created_at ?? timestamp,
            updated_at: timestamp,
            expires_at: new Date(Date.now() + settings.memory.working_memory_retention_hours * 60 * 60 * 1000).toISOString(),
          },
          ...workingMemory.filter((item) => item.id !== "working_identity_qpaw_name"),
        ];
      }
      const assistant = {
        role: "assistant" as const,
        content: extractedName
          ? `知道了，我叫 ${extractedName}。`
          : settings.llm.api_key
          ? "收到。我会按低打扰模式记住这件事。"
          : "我还没有 LLM API Key，所以先用本地回复陪你工作。",
        created_at: now(),
      };
      chatHistory = [...chatHistory, user, assistant];
      if (/记住|remember/i.test(content)) {
        memories = [{ body: content, source: "chat", created_at: now() }, ...memories];
      }
      const memoryDecision =
        /记住|记得|记一下|以后提醒我|remember|note that/i.test(content)
          ? {
              action: "save" as const,
              reason: "explicit_memory_request",
              tags: ["explicit_memory_request"],
              confirmation_prompt: null,
            }
          : /睡不好|睡眠|疲惫|很累|没精神|焦虑|压力|疼|不舒服|肩颈|头痛|胃/.test(content)
            ? {
                action: "ask" as const,
                reason: "possible_personal_state_signal",
                tags: ["personal_state"],
                confirmation_prompt: "这件事以后可能有用，要我记一下吗？",
              }
            : {
                action: "ignore" as const,
                reason: "no_memory_signal",
                tags: [],
                confirmation_prompt: null,
              };
      return { user, assistant, memories, memory_decision: memoryDecision } satisfies SendChatResponse as T;
    }
    case "list_chat_history":
      return structuredClone(chatHistory) as T;
    case "list_working_memory":
      return structuredClone(workingMemory) as T;
    case "clear_working_memory":
      workingMemory = [];
      return undefined as T;
    case "list_memories":
      return structuredClone(memories) as T;
    case "clear_memory":
      memories = [];
      chatHistory = [];
      layeredMemories = [];
      workingMemory = [];
      return undefined as T;
    case "list_memory_items":
      void (args?.filter as MemoryLayerFilter | undefined);
      return structuredClone(layeredMemories) as T;
    case "delete_memory_item":
      layeredMemories = layeredMemories.filter((item) => item.id !== args?.id);
      return undefined as T;
    case "query_memory":
      return { items: structuredClone(layeredMemories), context: "" } as T;
    case "run_memory_consolidation": {
      const report: MemoryConsolidationReport = {
        message: settings.llm.api_key ? "浏览器预览模式不会写入真实后端" : "LLM 未配置，浏览器预览模式只显示占位状态",
        job: {
          id: crypto.randomUUID(),
          date: new Date().toISOString().slice(0, 10),
          status: settings.llm.api_key ? "completed" : "pending",
          input_event_count: memories.length,
          created_count: 0,
          updated_count: 0,
          archived_count: 0,
          error: settings.llm.api_key ? null : "LLM is not configured",
          started_at: now(),
          finished_at: now(),
        },
      };
      return report as T;
    }
    case "get_memory_stats": {
      const stats: MemoryStats = {
        raw_events: memories.length,
        l0: layeredMemories.filter((item) => item.layer === "l0").length,
        l1_concepts: layeredMemories.filter((item) => item.layer === "l1_concept").length,
        l1_relations: layeredMemories.filter((item) => item.layer === "l1_relation").length,
        l2_events: layeredMemories.filter((item) => item.layer === "l2").length,
        l3_reflections: layeredMemories.filter((item) => item.layer === "l3").length,
        pending_jobs: 0,
        last_consolidation_at: null,
      };
      return stats as T;
    }
    case "record_task_event":
      memories = [
        {
          body: String(args?.summary ?? ""),
          source: "task",
          created_at: now(),
        },
        ...memories,
      ];
      return undefined as T;
    case "trigger_test_reminder":
      const reminderKind = String(args?.kind ?? "hydration");
      const reminder = settings.reminders.items.find((item) => item.id === reminderKind) ?? settings.reminders.items[0];
      return {
        id: crypto.randomUUID(),
        kind: reminder?.id ?? reminderKind,
        title: reminder?.title ?? "测试提醒",
        message: `测试提醒：${warmReminderMessage(reminder?.title ?? "休息一下")}`,
        action_label: reminder?.action_label ?? "完成",
        due_at: now(),
      } satisfies ReminderPayload as T;
    case "get_reminder_status": {
      const elapsedSeconds = Math.floor((Date.now() - fallbackStartedAt) / 1000);
      const status: ReminderRuntimeStatus = {
        paused: settings.reminders.paused,
        idle_seconds: 0,
        active: true,
        items: settings.reminders.items.map((item) => ({
          id: item.id,
          title: item.title,
          paused: item.paused,
          active_seconds: Math.min(elapsedSeconds, item.interval_minutes * 60),
          interval_seconds: item.interval_minutes * 60,
          idle_grace_seconds: item.idle_grace_minutes * 60,
          pending_waited_seconds: null,
        })),
      };
      return status as T;
    }
    case "set_reminder_feedback":
      void (args as { payload?: ReminderFeedbackPayload });
      return undefined as T;
    default:
      throw new Error(`No fallback implemented for ${command}`);
  }
}

function isImagePath(path: string) {
  return /\.(png|jpe?g|webp)$/i.test(path);
}

function avatarName(path: string) {
  const filename = path.split(/[\\/]/).pop() || "Imported avatar";
  return filename.replace(/\.model3\.json$/i, "").replace(/\.(png|jpe?g|webp)$/i, "");
}

function extractName(message: string) {
  const match = message.match(/(?:你叫|你名字是|你的名字是|QPaw\s*叫|qpaw\s*叫|它叫|名字是)\s*[:：]?["“']?([^\s，,。.!！?？；;、"”']+)/);
  return match?.[1] ?? null;
}

function warmReminderMessage(title: string) {
  if (title.includes("水") || title.toLowerCase().includes("drink")) {
    return "该喝点水了。慢慢喝几口，让身体从安静的地方重新亮起来。";
  }
  if (title.includes("眼") || title.includes("休息") || title.toLowerCase().includes("eye")) {
    return "眼睛也陪你努力很久了。闭上眼休息一小会儿，把清亮还给自己。";
  }
  return `到「${title || "休息一下"}」的时间了。先把自己照顾好一点，身体会记得每一次温柔的停顿。`;
}
