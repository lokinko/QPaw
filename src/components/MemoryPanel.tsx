import { Brain, RefreshCw, Search, Trash2, XCircle } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ControlButton } from "./ControlButton";
import { debugError, debugLog, formatError } from "../lib/debug";
import { api } from "../lib/tauri";
import type {
  LayeredMemoryItem,
  MemoryL0Category,
  MemoryLayer,
  MemoryStats,
  WorkingMemoryItem,
} from "../lib/types";

const layerOptions: Array<{ value: ""; label: string } | { value: MemoryLayer; label: string }> = [
  { value: "", label: "全部层级" },
  { value: "l0", label: "L0 用途" },
  { value: "l1_concept", label: "L1 概念" },
  { value: "l1_relation", label: "L1 关系" },
  { value: "l2", label: "L2 事件" },
  { value: "l3", label: "L3 经验" },
];

const categoryOptions: Array<{ value: ""; label: string } | { value: MemoryL0Category; label: string }> = [
  { value: "", label: "全部分类" },
  { value: "preference", label: "偏好" },
  { value: "person_relation", label: "人物/关系" },
  { value: "task_project", label: "任务/项目" },
  { value: "health_habit", label: "健康习惯" },
  { value: "interaction_style", label: "交互风格" },
  { value: "lesson", label: "经验教训" },
];

const layerLabel: Record<MemoryLayer, string> = {
  l0: "L0",
  l1_concept: "L1 概念",
  l1_relation: "L1 关系",
  l2: "L2",
  l3: "L3",
};

export function MemoryPanel() {
  const [items, setItems] = useState<LayeredMemoryItem[]>([]);
  const [workingItems, setWorkingItems] = useState<WorkingMemoryItem[]>([]);
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [layer, setLayer] = useState<MemoryLayer | "">("");
  const [category, setCategory] = useState<MemoryL0Category | "">("");
  const [query, setQuery] = useState("");
  const [includeArchived, setIncludeArchived] = useState(false);
  const [status, setStatus] = useState("分层记忆会在每日 24:00 自动沉淀。");
  const [isLoading, setIsLoading] = useState(false);

  const filter = useMemo(
    () => ({
      layer: layer || null,
      category: category || null,
      query: query.trim() || null,
      include_archived: includeArchived,
    }),
    [category, includeArchived, layer, query],
  );

  const refresh = async () => {
    setIsLoading(true);
    debugLog("memory-panel:refresh:start", {
      layer: filter.layer,
      category: filter.category,
      queryLen: filter.query?.length ?? 0,
      includeArchived: filter.include_archived,
    });
    try {
      const [nextItems, nextStats] = await Promise.all([
        api.listMemoryItems(filter),
        api.getMemoryStats(),
      ]);
      const nextWorkingItems = await api.listWorkingMemory();
      debugLog("memory-panel:refresh:ok", {
        itemCount: nextItems.length,
        workingItemCount: nextWorkingItems.length,
        rawEvents: nextStats.raw_events,
        pendingJobs: nextStats.pending_jobs,
      });
      setItems(nextItems);
      setWorkingItems(nextWorkingItems);
      setStats(nextStats);
    } catch (error) {
      debugError("memory-panel:refresh:failed", error);
      setStatus(`记忆读取失败：${formatError(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, [filter]);

  const consolidate = async () => {
    setIsLoading(true);
    debugLog("memory-panel:consolidate:start");
    try {
      const report = await api.runMemoryConsolidation();
      debugLog("memory-panel:consolidate:ok", {
        status: report.job.status,
        inputEvents: report.job.input_event_count,
        created: report.job.created_count,
        updated: report.job.updated_count,
        archived: report.job.archived_count,
      });
      setStatus(`${report.message}：${report.job.status}`);
      await refresh();
    } catch (error) {
      debugError("memory-panel:consolidate:failed", error);
      setStatus(`记忆沉淀失败：${formatError(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const remove = async (item: LayeredMemoryItem) => {
    debugLog("memory-panel:remove:start", { layer: item.layer, id: item.id });
    try {
      await api.deleteMemoryItem(item.layer, item.id);
      debugLog("memory-panel:remove:ok", { layer: item.layer, id: item.id });
      setStatus(`已删除 ${item.title}`);
      await refresh();
    } catch (error) {
      debugError("memory-panel:remove:failed", error, { layer: item.layer, id: item.id });
      setStatus(`删除失败：${formatError(error)}`);
    }
  };

  const clearWorking = async () => {
    debugLog("memory-panel:clear-working:start");
    try {
      await api.clearWorkingMemory();
      debugLog("memory-panel:clear-working:ok");
      setStatus("已清空今日工作记忆");
      await refresh();
    } catch (error) {
      debugError("memory-panel:clear-working:failed", error);
      setStatus(`清空工作记忆失败：${formatError(error)}`);
    }
  };

  return (
    <section className="settings-card memory-panel">
      <header>
        <h2>分层记忆</h2>
        <p>只读审计 L0-L3 记忆，支持搜索、删除和手动沉淀。</p>
      </header>

      <div className="memory-stats">
        <span>工作 {workingItems.length}</span>
        <span>原始 {stats?.raw_events ?? 0}</span>
        <span>L0 {stats?.l0 ?? 0}</span>
        <span>L1 {((stats?.l1_concepts ?? 0) + (stats?.l1_relations ?? 0))}</span>
        <span>L2 {stats?.l2_events ?? 0}</span>
        <span>L3 {stats?.l3_reflections ?? 0}</span>
        <span>待处理 {stats?.pending_jobs ?? 0}</span>
      </div>

      <div className="memory-toolbar">
        <label>
          层级
          <select value={layer} onChange={(event) => setLayer(event.target.value as MemoryLayer | "")}>
            {layerOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          分类
          <select
            value={category}
            onChange={(event) => setCategory(event.target.value as MemoryL0Category | "")}
          >
            {categoryOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label className="memory-search">
          搜索
          <span>
            <Search size={15} />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="偏好、人物、任务、经验"
            />
          </span>
        </label>
      </div>

      <div className="button-row">
        <ControlButton icon={<RefreshCw size={16} />} onClick={() => void refresh()}>
          刷新
        </ControlButton>
        <ControlButton icon={<Brain size={16} />} variant="primary" onClick={() => void consolidate()}>
          立即沉淀今日记忆
        </ControlButton>
        <ControlButton icon={<XCircle size={16} />} onClick={() => void clearWorking()}>
          清空工作记忆
        </ControlButton>
        <label className="memory-checkbox">
          <input
            type="checkbox"
            checked={includeArchived}
            onChange={(event) => setIncludeArchived(event.target.checked)}
          />
          显示归档
        </label>
      </div>

      <p className="status-line">{isLoading ? "正在读取记忆..." : status}</p>

      <div className="memory-list">
        <h3 className="memory-subtitle">今日工作记忆</h3>
        {workingItems.length === 0 ? (
          <p className="empty-copy">今天还没有可用的工作记忆。</p>
        ) : (
          workingItems.map((item) => (
            <article key={item.id} className="memory-item">
              <div>
                <span>{item.kind}</span>
                <span>{Math.round(item.confidence * 100)}%</span>
              </div>
              <h3>{item.title}</h3>
              <p>{item.summary}</p>
              {item.keywords.length > 0 ? (
                <footer>
                  {item.keywords.slice(0, 5).map((keyword) => (
                    <span key={keyword}>{keyword}</span>
                  ))}
                </footer>
              ) : null}
            </article>
          ))
        )}
      </div>

      <div className="memory-list">
        <h3 className="memory-subtitle">长期分层记忆</h3>
        {items.length === 0 ? (
          <p className="empty-copy">还没有分层记忆。聊天或提醒反馈后，可手动沉淀或等待每日 24:00。</p>
        ) : (
          items.map((item) => (
            <article key={`${item.layer}-${item.id}`} className="memory-item">
              <div>
                <span>{layerLabel[item.layer]}</span>
                {item.category ? <span>{item.category}</span> : null}
                <span>{item.status}</span>
              </div>
              <h3>{item.title}</h3>
              <p>{item.summary}</p>
              {item.tags.length > 0 ? (
                <footer>
                  {item.tags.slice(0, 5).map((tag) => (
                    <span key={tag}>{tag}</span>
                  ))}
                </footer>
              ) : null}
              <button title="删除记忆" onClick={() => void remove(item)}>
                <Trash2 size={15} />
              </button>
            </article>
          ))
        )}
      </div>
    </section>
  );
}
