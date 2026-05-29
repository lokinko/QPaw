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

export function replyPreviewClassName(pinned: boolean, hoverDismissed: boolean) {
  return [
    "pet-reply-wrap",
    pinned ? "is-pinned" : null,
    hoverDismissed ? "is-hover-dismissed" : null,
  ]
    .filter(Boolean)
    .join(" ");
}

export function PetWindow() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [reminder, setReminder] = useState<ReminderPayload | null>(null);
  const [chatInput, setChatInput] = useState("");
  const [reply, setReply] = useState("我在旁边，不会打断你。");
  const [replyPreviewPinned, setReplyPreviewPinned] = useState(false);
  const [replyPreviewHoverDismissed, setReplyPreviewHoverDismissed] = useState(false);

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
    setReplyPreviewPinned(false);
    setReplyPreviewHoverDismissed(false);
    debugLog("pet-window:send:start", { messageLen: message.length });
    try {
      const response = await api.sendChatMessage(message);
      debugLog("pet-window:send:ok", { assistantLen: response.assistant.content.length });
      setReply(
        response.memory_decision?.confirmation_prompt
          ? `${response.assistant.content}\n${response.memory_decision.confirmation_prompt}`
          : response.assistant.content,
      );
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
        <div
          className={replyPreviewClassName(replyPreviewPinned, replyPreviewHoverDismissed)}
          onMouseLeave={() => setReplyPreviewHoverDismissed(false)}
        >
          <button
            className="pet-chat-dock__reply"
            type="button"
            aria-label="查看完整回复"
            aria-expanded={replyPreviewPinned}
            onClick={() => {
              setReplyPreviewHoverDismissed(false);
              setReplyPreviewPinned((current) => !current);
            }}
          >
            {reply}
          </button>
          <div className="pet-reply-preview" role="dialog" aria-label="完整回复预览">
            <button
              className="pet-reply-preview__close"
              type="button"
              aria-label="关闭完整回复预览"
              onClick={() => {
                setReplyPreviewPinned(false);
                setReplyPreviewHoverDismissed(true);
              }}
            >
              ×
            </button>
            <p>{reply}</p>
          </div>
        </div>
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
