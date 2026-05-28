import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-dialog";
import { debugError, debugLog } from "./debug";
import { invokeFallback } from "./fallback";
import type {
  AppSettings,
  AvatarManifest,
  ChatMessage,
  CodexDevStatus,
  LayeredMemoryItem,
  MemoryConsolidationReport,
  MemoryDocument,
  MemoryLayer,
  MemoryLayerFilter,
  MemoryQueryRequest,
  MemoryQueryResponse,
  MemoryStats,
  ReminderFeedbackPayload,
  ReminderKind,
  ReminderPayload,
  ReminderRuntimeStatus,
  SendChatResponse,
  WorkingMemoryItem,
} from "./types";

export type ResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";

const isTauri = () => "__TAURI_INTERNALS__" in window;

export const isTauriRuntime = isTauri;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const startedAt = performance.now();
  debugLog("tauri:invoke:start", {
    command,
    runtime: isTauri() ? "tauri" : "fallback",
    argKeys: Object.keys(args ?? {}),
  });
  if (!isTauri()) {
    try {
      const result = await invokeFallback<T>(command, args);
      debugLog("tauri:invoke:ok", {
        command,
        runtime: "fallback",
        elapsedMs: Math.round(performance.now() - startedAt),
      });
      return result;
    } catch (error) {
      debugError("tauri:invoke:failed", error, {
        command,
        runtime: "fallback",
        elapsedMs: Math.round(performance.now() - startedAt),
      });
      throw error;
    }
  }
  try {
    const result = await invoke<T>(command, args);
    debugLog("tauri:invoke:ok", {
      command,
      runtime: "tauri",
      elapsedMs: Math.round(performance.now() - startedAt),
    });
    return result;
  } catch (error) {
    debugError("tauri:invoke:failed", error, {
      command,
      runtime: "tauri",
      elapsedMs: Math.round(performance.now() - startedAt),
    });
    throw error;
  }
}

export const api = {
  getSettings: () => call<AppSettings>("get_settings"),
  getCodexDevStatus: () => call<CodexDevStatus>("get_codex_dev_status"),
  saveSettings: (settings: AppSettings) => call<AppSettings>("save_settings", { settings }),
  savePetWindowSize: (width: number, height: number) =>
    call<void>("save_pet_window_size", { width, height }),
  importAvatar: (path: string) => call<AvatarManifest>("import_avatar", { path }),
  sendChatMessage: (message: string) => call<SendChatResponse>("send_chat_message", { message }),
  listChatHistory: () => call<ChatMessage[]>("list_chat_history"),
  listWorkingMemory: () => call<WorkingMemoryItem[]>("list_working_memory"),
  clearWorkingMemory: () => call<void>("clear_working_memory"),
  queryMemory: (request: MemoryQueryRequest) =>
    call<MemoryQueryResponse>("query_memory", { request }),
  listMemoryItems: (filter: MemoryLayerFilter) =>
    call<LayeredMemoryItem[]>("list_memory_items", { filter }),
  deleteMemoryItem: (layer: MemoryLayer, id: string) =>
    call<void>("delete_memory_item", { layer, id }),
  runMemoryConsolidation: (date?: string | null) =>
    call<MemoryConsolidationReport>("run_memory_consolidation", { date: date ?? null }),
  getMemoryStats: () => call<MemoryStats>("get_memory_stats"),
  recordTaskEvent: (summary: string, status?: string | null, content?: Record<string, unknown>) =>
    call<void>("record_task_event", { summary, status: status ?? null, content: content ?? null }),
  listMemories: () => call<MemoryDocument[]>("list_memories"),
  clearMemory: () => call<void>("clear_memory"),
  triggerTestReminder: (kind: ReminderKind) =>
    call<ReminderPayload>("trigger_test_reminder", { kind }),
  getReminderStatus: () => call<ReminderRuntimeStatus>("get_reminder_status"),
  setReminderFeedback: (payload: ReminderFeedbackPayload) =>
    call<void>("set_reminder_feedback", { payload }),
};

export async function pickAvatarAsset(): Promise<string | null> {
  if (!isTauri()) {
    return window.prompt("Path to .model3.json, .png, .jpg, .jpeg, or .webp file") || null;
  }

  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      { name: "Avatar Asset", extensions: ["json", "png", "jpg", "jpeg", "webp"] },
      { name: "Live2D Model", extensions: ["json"] },
      { name: "Static Image", extensions: ["png", "jpg", "jpeg", "webp"] },
    ],
  });

  return typeof selected === "string" ? selected : null;
}

export function toAssetUrl(path: string | null): string | null {
  if (!path) return null;
  if (/^(?:https?:|data:|blob:|\/)/i.test(path)) return path;
  if (!isTauri()) return path;
  return convertFileSrc(path);
}

export async function openSettingsWindow() {
  if (!isTauri()) {
    window.location.href = "/?view=settings";
    return;
  }

  const settings = await WebviewWindow.getByLabel("settings");
  await settings?.show();
  await settings?.setFocus();
}

export async function startWindowResize(direction: ResizeDirection) {
  if (!isTauri()) return;
  await getCurrentWindow().startResizeDragging(direction);
}

export async function currentWindowLogicalSize() {
  if (!isTauri()) return null;
  const window = getCurrentWindow();
  const [size, scaleFactor] = await Promise.all([window.innerSize(), window.scaleFactor()]);
  const logical = size.toLogical(scaleFactor);
  return { width: logical.width, height: logical.height };
}

export async function watchCurrentWindowSize(
  onResize: () => void,
  onCloseRequested?: () => void | Promise<void>,
) {
  if (!isTauri()) return () => undefined;
  const window = getCurrentWindow();
  const unlistenResize = await window.onResized(onResize);
  const unlistenClose = onCloseRequested
    ? await window.onCloseRequested(() => onCloseRequested())
    : undefined;

  return () => {
    unlistenResize();
    unlistenClose?.();
  };
}

export function onReminderDue(handler: (payload: ReminderPayload) => void) {
  if (!isTauri()) {
    return Promise.resolve(() => undefined);
  }
  return listen<ReminderPayload>("reminder_due", (event) => handler(event.payload));
}
