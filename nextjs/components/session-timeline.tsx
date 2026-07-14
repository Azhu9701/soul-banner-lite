"use client";

import { useEffect, useState, useRef } from "react";
import { Link } from "@tanstack/react-router";
import { Trash2, Brain, Users, ChevronRight, Download, CheckSquare, Square, X } from "lucide-react";
import { ConfirmButton } from "@/components/ui/confirm-button";
import { deleteSession, exportSessionMarkdown, fetchSessions, batchDeleteSessions } from "@/lib/api";
import { triggerSessionsUpdate, SESSIONS_UPDATED_EVENT } from "@/components/sidebar-sessions";
import type { SessionSummary } from "@/lib/api";
import { modeLabel, MODE_COLORS_BG, MODE_COLORS_TEXT } from "@/config/possession-modes";
import type { PossessionMode } from "@/config/possession-modes";

interface SessionTimelineProps {
  sessions: SessionSummary[];
}

const modeBgColors: Record<string, string> = {
  single: "bg-blue-50 dark:bg-blue-950/30 border-blue-200 dark:border-blue-800",
  conference: "bg-purple-50 dark:bg-purple-950/30 border-purple-200 dark:border-purple-800",
  debate: "bg-orange-50 dark:bg-orange-950/30 border-orange-200 dark:border-orange-800",
  relay: "bg-green-50 dark:bg-green-950/30 border-green-200 dark:border-green-800",
  learn: "bg-teal-50 dark:bg-teal-950/30 border-teal-200 dark:border-teal-800",
  practice_opening: "bg-red-50 dark:bg-red-950/30 border-red-200 dark:border-red-800",
};

function groupByDate(sessions: SessionSummary[]): Map<string, SessionSummary[]> {
  const groups = new Map<string, SessionSummary[]>();
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86400000);

  for (const s of sessions) {
    const d = new Date(s.created_at);
    const dateOnly = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    let label: string;
    if (dateOnly.getTime() === today.getTime()) {
      label = "今天";
    } else if (dateOnly.getTime() === yesterday.getTime()) {
      label = "昨天";
    } else {
      label = d.toLocaleDateString("zh-CN", { month: "short", day: "numeric" });
    }
    const arr = groups.get(label) || [];
    arr.push(s);
    groups.set(label, arr);
  }
  return groups;
}

function SessionRow({
  s,
  onDelete,
  selectMode,
  selected,
  onToggleSelect,
}: {
  s: SessionSummary;
  onDelete: (id: string) => void;
  selectMode: boolean;
  selected: boolean;
  onToggleSelect: (id: string) => void;
}) {
  const handleExport = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    try { await exportSessionMarkdown(s.id, s.title); }
    catch { }
  };

  const handleDelete = async () => {
    try {
      await deleteSession(s.id);
      onDelete(s.id);
      triggerSessionsUpdate();
    } catch {}
  };

  const handleClick = (e: React.MouseEvent) => {
    if (selectMode) {
      e.preventDefault();
      e.stopPropagation();
      onToggleSelect(s.id);
    }
  };

  const handleCheckboxClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onToggleSelect(s.id);
  };

  return (
    <Link
      to={`/sessions/${s.id}` as any}
      onClick={handleClick}
      className={`flex items-center gap-3 p-3 rounded-lg border transition-all hover:shadow-md group relative ${
        selectMode && selected ? "ring-2 ring-primary bg-primary/5" : ""
      } ${selectMode ? "" : modeBgColors[s.mode] || "bg-background border-muted"}`}
      data-testid={`session-item-${s.id}`}
    >
      {/* Checkbox in select mode */}
      {selectMode && (
        <div
          className="shrink-0 cursor-pointer"
          onClick={handleCheckboxClick}
        >
          {selected ? (
            <CheckSquare className="h-5 w-5 text-primary" />
          ) : (
            <Square className="h-5 w-5 text-muted-foreground" />
          )}
        </div>
      )}

      {/* Mode indicator */}
      <div className="flex items-center gap-2 shrink-0">
        <div className={`w-2 h-2 rounded-full ${MODE_COLORS_BG[s.mode as PossessionMode] || "bg-gray-400"}`} />
        <span className={`text-xs font-medium px-2 py-0.5 rounded-full bg-white/50 dark:bg-gray-800 ${MODE_COLORS_TEXT[s.mode as PossessionMode] ? '' : 'text-muted-foreground'}`}>
          {modeLabel(s.mode)}
        </span>
      </div>

      {/* Title & meta */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-medium truncate group-hover:text-primary transition-colors">
            {s.title}
          </h4>
          {s.observation_count > 0 && (
            <span
              className="text-[10px] text-blue-500/80 shrink-0 px-1 rounded bg-blue-500/10"
              title={`${s.observation_count} 条压缩摘要`}
            >
              📝{s.observation_count}
            </span>
          )}
        </div>
        {s.digest_summary && (
          <div className="text-xs text-muted-foreground/80 truncate mt-0.5 italic">
            “{s.digest_summary}”
          </div>
        )}
        <div className="flex items-center gap-3 text-xs text-muted-foreground mt-0.5">
          {s.message_count > 0 && <span>{s.message_count} 条消息</span>}
          {s.total_tokens > 0 && <span>{s.total_tokens.toLocaleString()} tokens</span>}
        </div>
      </div>

      {/* Soul count */}
      <div className="flex items-center gap-1 text-xs text-muted-foreground shrink-0">
        <Users className="h-3 w-3" />
        <span>{s.soul_count || 1}</span>
      </div>

      {/* Time */}
      <div className="text-xs text-muted-foreground shrink-0 text-right w-16">
        {new Date(s.created_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
      </div>

      {/* Hover actions (hidden in select mode) */}
      {!selectMode && (
        <div className="opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1 shrink-0">
          <button
            onClick={handleExport}
            className="p-1 hover:bg-muted rounded"
            title="导出 Markdown"
          >
            <Download className="h-4 w-4" />
          </button>
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
          <ConfirmButton
            icon={<Trash2 className="h-4 w-4" />}
            confirmText="确认"
            title="删除会话"
            className="shrink-0"
            onConfirm={handleDelete}
          />
        </div>
      )}
    </Link>
  );
}

export function SessionTimeline({ sessions: initialSessions }: SessionTimelineProps) {
  const [sessions, setSessions] = useState<SessionSummary[]>(initialSessions);
  const [selectMode, setSelectMode] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [batchDeleting, setBatchDeleting] = useState(false);
  const skipEventRef = useRef(false);

  useEffect(() => {
    const handle = () => {
      if (skipEventRef.current) {
        skipEventRef.current = false;
        return;
      }
      fetchSessions(200).then(setSessions).catch(() => {});
    };
    window.addEventListener(SESSIONS_UPDATED_EVENT, handle);
    return () => window.removeEventListener(SESSIONS_UPDATED_EVENT, handle);
  }, []);

  const handleDelete = (id: string) => {
    skipEventRef.current = true;
    setSessions((prev) => prev.filter((s) => s.id !== id));
  };

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectAll = () => {
    setSelected(new Set(sessions.map((s) => s.id)));
  };

  const deselectAll = () => {
    setSelected(new Set());
  };

  const enterSelectMode = () => {
    setSelectMode(true);
    setSelected(new Set());
  };

  const exitSelectMode = () => {
    setSelectMode(false);
    setSelected(new Set());
  };

  const handleBatchDelete = async () => {
    if (selected.size === 0) return;
    setBatchDeleting(true);
    try {
      const ids = Array.from(selected);
      await batchDeleteSessions(ids);
      skipEventRef.current = true;
      setSessions((prev) => prev.filter((s) => !selected.has(s.id)));
      setSelected(new Set());
      setSelectMode(false);
      triggerSessionsUpdate();
    } catch (e) {
      console.error("批量删除失败:", e);
    } finally {
      setBatchDeleting(false);
    }
  };

  if (sessions.length === 0) {
    return (
      <div data-testid="session-timeline" className="flex flex-col items-center justify-center py-12">
        <Brain className="h-12 w-12 text-muted-foreground mb-4" />
        <h3 className="text-sm font-semibold text-muted-foreground">暂无会话记录</h3>
        <p className="text-xs text-muted-foreground mt-1">开始你的第一次庭审之旅</p>
        <Link to="/possess" className="mt-4 text-xs text-primary hover:underline">
          前往庭审
        </Link>
      </div>
    );
  }

  const groups = groupByDate(sessions);

  return (
    <div data-testid="session-timeline" className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-primary" />
          <h3 className="text-sm font-semibold">会话历史</h3>
        </div>

        <div className="flex items-center gap-2">
          {selectMode ? (
            <>
              <button
                onClick={selected.size === sessions.length ? deselectAll : selectAll}
                className="text-xs text-muted-foreground hover:text-foreground transition-colors"
              >
                {selected.size === sessions.length ? "取消全选" : "全选"}
              </button>
              <ConfirmButton
                icon={<Trash2 className="h-4 w-4" />}
                confirmText={`删除 ${selected.size}`}
                title={`批量删除 ${selected.size} 个会话`}
                disabled={selected.size === 0 || batchDeleting}
                className="text-xs h-8"
                onConfirm={handleBatchDelete}
              />
              <button
                onClick={exitSelectMode}
                className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
              >
                <X className="h-3.5 w-3.5" />
                退出
              </button>
            </>
          ) : (
            <>
              <span className="text-xs text-muted-foreground">{sessions.length} 个会话</span>
              <button
                onClick={enterSelectMode}
                className="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
              >
                <CheckSquare className="h-3.5 w-3.5" />
                批量选择
              </button>
            </>
          )}
        </div>
      </div>

      <div className="space-y-4">
        {Array.from(groups.entries()).map(([label, items]) => (
          <div key={label}>
            <h4 className="text-xs font-semibold text-muted-foreground mb-2 px-1">
              {label}
            </h4>
            <div className="space-y-2">
              {items.map((s) => (
                <div key={s.id} className="relative">
                  <SessionRow
                    s={s}
                    onDelete={handleDelete}
                    selectMode={selectMode}
                    selected={selected.has(s.id)}
                    onToggleSelect={toggleSelect}
                  />
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
