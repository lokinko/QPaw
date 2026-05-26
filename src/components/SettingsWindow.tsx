import { Download, Eraser, FolderOpen, Pause, Play, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { ChatPanel } from "./ChatPanel";
import { ControlButton } from "./ControlButton";
import { MemoryPanel } from "./MemoryPanel";
import { api, isTauriRuntime, pickAvatarAsset } from "../lib/tauri";
import { fallbackSettings } from "../lib/fallback";
import {
  DEFAULT_BUILT_IN_AVATAR_ID,
  type AppSettings,
  type ReminderDefinition,
  type ReminderRuntimeStatus,
} from "../lib/types";

export function SettingsWindow() {
  const [settings, setSettings] = useState<AppSettings>(fallbackSettings);
  const [status, setStatus] = useState("设置会自动保存到本机应用数据目录。");
  const [reminderStatus, setReminderStatus] = useState<ReminderRuntimeStatus | null>(null);

  const refreshReminderStatus = useCallback(async () => {
    const current = await api.getReminderStatus();
    setReminderStatus(current);
    return current;
  }, []);

  useEffect(() => {
    void api.getSettings().then(setSettings);
    void refreshReminderStatus();
    const timer = window.setInterval(() => {
      void refreshReminderStatus();
    }, 1000);
    return () => window.clearInterval(timer);
  }, [refreshReminderStatus]);

  const save = async (next: AppSettings) => {
    setSettings(next);
    await api.saveSettings(next);
    void refreshReminderStatus();
    setStatus("已保存");
  };

  const llmReady = useMemo(
    () => Boolean(settings.llm.base_url && settings.llm.model && settings.llm.api_key),
    [settings.llm],
  );
  const desktopRuntime = isTauriRuntime();

  const updateReminderPaused = (paused: boolean) =>
    save({ ...settings, reminders: { ...settings.reminders, paused } });

  const updateReminderItem = (id: string, patch: Partial<ReminderDefinition>) =>
    save({
      ...settings,
      reminders: {
        ...settings.reminders,
        items: settings.reminders.items.map((item) => (item.id === id ? { ...item, ...patch } : item)),
      },
    });

  const addReminder = () =>
    save({
      ...settings,
      reminders: {
        ...settings.reminders,
        items: [
          ...settings.reminders.items,
          {
            id: `custom_${crypto.randomUUID()}`,
            title: "新提醒",
            message: "到时间了，轻轻提醒一下。",
            action_label: "照顾好了",
            interval_minutes: 30,
            idle_grace_minutes: 5,
            paused: false,
          },
        ],
      },
    });

  const deleteReminder = (id: string) =>
    save({
      ...settings,
      reminders: {
        ...settings.reminders,
        items: settings.reminders.items.filter((item) => item.id !== id),
      },
    });

  const importAvatar = async () => {
    const path = await pickAvatarAsset();
    if (!path) return;
    const manifest = await api.importAvatar(path);
    await save({
      ...settings,
      avatar: {
        ...settings.avatar,
        current_avatar_id: manifest.id,
        kind: manifest.kind,
        model_json_path: manifest.model_json_path,
        image_path: manifest.image_path,
      },
    });
    setStatus(`已导入 ${manifest.name}`);
  };
  const useBuiltInAvatar = () =>
    save({
      ...settings,
      avatar: {
        ...settings.avatar,
        current_avatar_id: DEFAULT_BUILT_IN_AVATAR_ID,
        kind: "built_in",
        model_json_path: null,
        image_path: null,
      },
    });
  const avatarPath =
    settings.avatar.kind === "image"
      ? settings.avatar.image_path
      : settings.avatar.kind === "live2d"
        ? settings.avatar.model_json_path
        : null;
  const avatarLabel =
    settings.avatar.kind === "image"
      ? "静态图片"
      : settings.avatar.kind === "live2d"
        ? "Live2D 模型"
        : "内置动态形象";

  const exportMemory = async () => {
    const memories = await api.listMemories();
    const blob = new Blob([JSON.stringify(memories, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `qpaw-memory-${new Date().toISOString().slice(0, 10)}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    setStatus("记忆已导出为 JSON");
  };

  const clearMemory = async () => {
    await api.clearMemory();
    setStatus("已清空本地对话、记忆和行为事件");
  };

  return (
    <main className="settings-window">
      <header className="settings-header">
        <div>
          <h1>QPaw</h1>
          <p>低打扰桌面宠物，优先本地规则，必要时连接 LLM。</p>
        </div>
        <span className={llmReady ? "status-pill ready" : "status-pill"}>{llmReady ? "LLM 已配置" : "本地模式"}</span>
      </header>

      <div className="settings-grid">
        <section className="settings-card">
          <header>
            <h2>LLM 服务</h2>
            <p>OpenAI 兼容接口，提醒规则不依赖网络。</p>
          </header>
          <label>
            Base URL
            <input
              value={settings.llm.base_url}
              onChange={(event) =>
                save({ ...settings, llm: { ...settings.llm, base_url: event.target.value } })
              }
              placeholder="https://api.openai.com/v1"
            />
          </label>
          <label>
            Model
            <input
              value={settings.llm.model}
              onChange={(event) =>
                save({ ...settings, llm: { ...settings.llm, model: event.target.value } })
              }
              placeholder="gpt-4.1-mini"
            />
          </label>
          <label>
            API Key
            <input
              type="password"
              value={settings.llm.api_key}
              onChange={(event) =>
                save({ ...settings, llm: { ...settings.llm, api_key: event.target.value } })
              }
              placeholder="只保存在本机明文配置中"
            />
          </label>
        </section>

        <section className="settings-card">
          <header>
            <h2>提醒节奏</h2>
            <p>
              {desktopRuntime
                ? "只记录活跃/空闲、提醒触发和反馈。"
                : "浏览器预览不会启动真实定时提醒，请用 Tauri 桌面进程验收。"}
            </p>
          </header>
          <div className="button-row">
            <ControlButton
              icon={settings.reminders.paused ? <Play size={16} /> : <Pause size={16} />}
              onClick={() => updateReminderPaused(!settings.reminders.paused)}
            >
              {settings.reminders.paused ? "恢复全部" : "暂停全部"}
            </ControlButton>
            <ControlButton icon={<RefreshCw size={16} />} onClick={() => void refreshReminderStatus()}>
              刷新状态
            </ControlButton>
            <ControlButton icon={<Plus size={16} />} variant="primary" onClick={() => void addReminder()}>
              新增提醒
            </ControlButton>
          </div>
          <div className="reminder-config-list compact">
            {settings.reminders.items.length === 0 ? (
              <p className="empty-copy">还没有提醒项。</p>
            ) : (
              settings.reminders.items.map((item) => (
                <article className="reminder-config-row" key={item.id}>
                  <label>
                    <span>标题</span>
                    <input
                      value={item.title}
                      onChange={(event) => updateReminderItem(item.id, { title: event.target.value })}
                    />
                  </label>
                  <label>
                    <span>间隔</span>
                    <input
                      type="number"
                      min={1}
                      value={item.interval_minutes}
                      onChange={(event) =>
                        updateReminderItem(item.id, { interval_minutes: Number(event.target.value) })
                      }
                    />
                  </label>
                  <label>
                    <span>宽限</span>
                    <input
                      type="number"
                      min={1}
                      value={item.idle_grace_minutes}
                      onChange={(event) =>
                        updateReminderItem(item.id, { idle_grace_minutes: Number(event.target.value) })
                      }
                    />
                  </label>
                  <div className="reminder-config-row__actions">
                    <ControlButton
                      icon={item.paused ? <Play size={16} /> : <Pause size={16} />}
                      onClick={() => updateReminderItem(item.id, { paused: !item.paused })}
                    >
                      {item.paused ? "恢复" : "暂停"}
                    </ControlButton>
                    <ControlButton icon={<Trash2 size={16} />} variant="danger" onClick={() => void deleteReminder(item.id)}>
                      删除
                    </ControlButton>
                  </div>
                </article>
              ))
            )}
          </div>
          <ReminderRuntimeSummary status={reminderStatus} />
        </section>

        <section className="settings-card">
          <header>
            <h2>桌面形象</h2>
            <p>支持内置动态形象、`.model3.json` Live2D 素材包，以及 png/jpg/webp 静态图片。</p>
          </header>
          <div className="avatar-import">
            <span>{avatarPath ? `${avatarLabel}：${avatarPath}` : avatarLabel}</span>
            <ControlButton icon={<FolderOpen size={16} />} variant="primary" onClick={() => void importAvatar()}>
              导入形象
            </ControlButton>
          </div>
          <div className="button-row">
            <ControlButton variant="primary" onClick={() => void useBuiltInAvatar()}>
              使用动态守夜猫
            </ControlButton>
          </div>
          <label>
            形象缩放
            <input
              type="range"
              min={0.6}
              max={1.6}
              step={0.05}
              value={settings.avatar.scale}
              onChange={(event) =>
                save({ ...settings, avatar: { ...settings.avatar, scale: Number(event.target.value) } })
              }
            />
          </label>
        </section>

        <section className="settings-card">
          <header>
            <h2>本地数据</h2>
            <p>开发简化策略：明文保存，可导出或清空。</p>
          </header>
          <div className="button-row">
            <ControlButton icon={<Download size={16} />} onClick={() => void exportMemory()}>
              导出记忆
            </ControlButton>
            <ControlButton icon={<Eraser size={16} />} variant="danger" onClick={() => void clearMemory()}>
              清空本地数据
            </ControlButton>
          </div>
          <p className="status-line">{status}</p>
        </section>

        <ChatPanel />
        <MemoryPanel />
      </div>
    </main>
  );
}

function ReminderRuntimeSummary({ status }: { status: ReminderRuntimeStatus | null }) {
  if (!status) {
    return <p className="status-line">提醒状态读取中...</p>;
  }

  return (
    <div className="reminder-status">
      <div>
        <span>运行状态</span>
        <strong>{status.paused ? "已暂停" : status.active ? "正在累计活跃时间" : "当前空闲"}</strong>
      </div>
      <div>
        <span>当前空闲</span>
        <strong>{formatDuration(status.idle_seconds)}</strong>
      </div>
      {status.items.length === 0 ? (
        <div className="reminder-status__wide">
          <span>提醒项</span>
          <strong>无</strong>
        </div>
      ) : (
        status.items.map((item) => (
          <div className="reminder-status__wide" key={item.id}>
            <span>{item.title}</span>
            <strong>
              {item.paused
                ? "已暂停"
                : item.pending_waited_seconds === null
                  ? progressText(item.active_seconds, item.interval_seconds)
                  : `待发，已等待 ${formatDuration(item.pending_waited_seconds)} / ${formatDuration(item.idle_grace_seconds)}`}
            </strong>
          </div>
        ))
      )}
    </div>
  );
}

function progressText(currentSeconds: number, totalSeconds: number) {
  return `${formatDuration(currentSeconds)} / ${formatDuration(totalSeconds)}`;
}

function formatDuration(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  if (minutes <= 0) return `${rest} 秒`;
  if (rest === 0) return `${minutes} 分钟`;
  return `${minutes} 分 ${rest} 秒`;
}
