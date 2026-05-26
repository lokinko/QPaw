import { Send } from "lucide-react";
import { useEffect, useState } from "react";
import { Live2DAvatar } from "./Live2DAvatar";
import { PixiNightCatAvatar } from "./PixiNightCatAvatar";
import { ReminderBubble } from "./ReminderBubble";
import { ResizeHandles } from "./ResizeHandles";
import { StaticAvatar } from "./StaticAvatar";
import { debugError, debugLog, formatError } from "../lib/debug";
import {
  api,
  currentWindowLogicalSize,
  isTauriRuntime,
  onReminderDue,
  watchCurrentWindowSize,
} from "../lib/tauri";
import { DEFAULT_AVATAR_IMAGE_PATH, type AppSettings, type ReminderPayload } from "../lib/types";

export function PetWindow() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [reminder, setReminder] = useState<ReminderPayload | null>(null);
  const [chatInput, setChatInput] = useState("");
  const [reply, setReply] = useState("我在旁边，不会打断你。");

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    void api.getSettings().then(setSettings);
    void onReminderDue(setReminder).then((unlisten) => {
      cleanup = unlisten;
    });
    return () => cleanup?.();
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let cleanup: (() => void) | undefined;
    let saveTimer: number | undefined;

    const persistCurrentSize = async () => {
      const size = await currentWindowLogicalSize();
      if (!size) return;

      const width = Math.round(size.width);
      const height = Math.round(size.height);
      await api.savePetWindowSize(width, height);
    };

    const flushSize = () => {
      if (saveTimer !== undefined) {
        window.clearTimeout(saveTimer);
        saveTimer = undefined;
      }
      return persistCurrentSize().catch((error) => {
        debugError("pet-window:size:save:failed", error);
      });
    };

    const scheduleSizeSave = () => {
      if (saveTimer !== undefined) {
        window.clearTimeout(saveTimer);
      }
      saveTimer = window.setTimeout(() => {
        saveTimer = undefined;
        void flushSize();
      }, 300);
    };

    void watchCurrentWindowSize(scheduleSizeSave, flushSize)
      .then((unlisten) => {
        cleanup = unlisten;
      })
      .catch((error) => {
        debugError("pet-window:size:watch:failed", error);
      });

    return () => {
      cleanup?.();
      if (saveTimer !== undefined) {
        window.clearTimeout(saveTimer);
      }
    };
  }, []);

  const sendQuickMessage = async () => {
    const message = chatInput.trim();
    if (!message) return;
    setChatInput("");
    debugLog("pet-window:send:start", { messageLen: message.length });
    try {
      const response = await api.sendChatMessage(message);
      debugLog("pet-window:send:ok", { assistantLen: response.assistant.content.length });
      setReply(response.assistant.content);
    } catch (error) {
      debugError("pet-window:send:failed", error, { messageLen: message.length });
      setReply(`发送失败：${formatError(error)}`);
    }
  };
  const avatar = settings?.avatar;
  const avatarKind = avatar?.kind ?? "built_in";
  const imagePath =
    avatarKind === "image"
      ? avatar?.image_path ?? DEFAULT_AVATAR_IMAGE_PATH
      : DEFAULT_AVATAR_IMAGE_PATH;

  return (
    <main className="pet-window" data-tauri-drag-region>
      <div className="pet-stage" data-tauri-drag-region>
        <ResizeHandles />

        {reminder ? <ReminderBubble reminder={reminder} onClose={() => setReminder(null)} /> : null}

        {avatarKind === "built_in" ? (
          <PixiNightCatAvatar scale={avatar?.scale ?? 1} />
        ) : avatarKind === "image" ? (
          <StaticAvatar imagePath={imagePath} scale={avatar?.scale ?? 1} />
        ) : (
          <Live2DAvatar modelPath={avatar?.model_json_path ?? null} scale={avatar?.scale ?? 1} />
        )}
      </div>
      <section className="pet-chat-dock" data-tauri-drag-region>
        <p data-tauri-drag-region>{reply}</p>
        <div className="pet-chat-dock__input">
          <input
            value={chatInput}
            onChange={(event) => setChatInput(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void sendQuickMessage();
            }}
            placeholder="和 QPaw 说一句"
            aria-label="和 QPaw 聊天"
          />
          <button title="发送" aria-label="发送" onClick={() => void sendQuickMessage()}>
            <Send size={16} />
          </button>
        </div>
      </section>
    </main>
  );
}
