import { RefreshCw, Send } from "lucide-react";
import { useEffect, useState } from "react";
import { ControlButton } from "./ControlButton";
import { debugError, debugLog, formatError } from "../lib/debug";
import { api } from "../lib/tauri";
import type { ChatMessage } from "../lib/types";

export function ChatPanel() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [status, setStatus] = useState("正在加载历史对话...");
  const [isSending, setIsSending] = useState(false);

  const loadHistory = async () => {
    debugLog("chat-panel:history:start");
    try {
      const history = await api.listChatHistory();
      debugLog("chat-panel:history:ok", { count: history.length });
      setMessages(history);
      setStatus(history.length > 0 ? `已加载 ${history.length} 条历史消息` : "还没有历史对话");
    } catch (error) {
      debugError("chat-panel:history:failed", error);
      setStatus(`历史读取失败：${formatError(error)}`);
    }
  };

  useEffect(() => {
    void loadHistory();
  }, []);

  const send = async () => {
    const message = input.trim();
    if (!message || isSending) return;
    setInput("");
    setIsSending(true);
    debugLog("chat-panel:send:start", { messageLen: message.length });
    try {
      const response = await api.sendChatMessage(message);
      debugLog("chat-panel:send:ok", {
        assistantLen: response.assistant.content.length,
        memories: response.memories.length,
      });
      setMessages((current) => [...current, response.user, response.assistant]);
      setStatus("已保存到本地历史对话");
    } catch (error) {
      debugError("chat-panel:send:failed", error, { messageLen: message.length });
      setMessages((current) => [
        ...current,
        { role: "user", content: message, created_at: new Date().toISOString() },
        {
          role: "assistant",
          content: `发送失败：${formatError(error)}`,
          created_at: new Date().toISOString(),
        },
      ]);
      setStatus("发送失败，未写入历史");
    } finally {
      setIsSending(false);
    }
  };

  return (
    <section className="settings-card chat-panel">
      <header>
        <h2>对话与记忆</h2>
        <p>聊天内容只写入本地文档库，LLM 失败时不会影响提醒。</p>
      </header>

      <div className="button-row">
        <ControlButton icon={<RefreshCw size={16} />} onClick={() => void loadHistory()}>
          刷新历史
        </ControlButton>
      </div>

      <p className="status-line">{status}</p>

      <div className="chat-log" aria-live="polite">
        {messages.length === 0 ? (
          <p className="empty-copy">还没有对话。你可以告诉它偏好的提醒方式，或让它记住一句话。</p>
        ) : (
          messages.map((message, index) => (
            <article key={`${message.created_at}-${index}`} className={`chat-message ${message.role}`}>
              <span>{message.role === "user" ? "你" : "QPaw"}</span>
              <p>{message.content}</p>
            </article>
          ))
        )}
      </div>

      <div className="chat-input">
        <input
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void send();
          }}
          placeholder="比如：记住我下午容易忘记喝水"
        />
        <ControlButton
          variant="primary"
          icon={<Send size={16} />}
          disabled={isSending}
          onClick={() => void send()}
        >
          {isSending ? "发送中" : "发送"}
        </ControlButton>
      </div>
    </section>
  );
}
