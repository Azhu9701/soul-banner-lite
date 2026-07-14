"use client";

import { useEffect, useState, useCallback } from "react";
import { Link, useLocation, useNavigate } from "@tanstack/react-router";
import { Trash2, Pencil, Check, X, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";
import { fetchSessions, deleteSession, renameSession, type SessionSummary } from "@/lib/api";
import { modeLabel, MODE_COLORS_TEXT, type PossessionMode } from "@/config/possession-modes";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ConfirmButton } from "@/components/ui/confirm-button";

export const SESSIONS_UPDATED_EVENT = "aionui-sessions-updated";

export function triggerSessionsUpdate() {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(SESSIONS_UPDATED_EVENT));
  }
}

export function SidebarSessions() {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [refreshing, setRefreshing] = useState(false);
  const [clientReady, setClientReady] = useState(false);
  const pathname = useLocation().pathname;
  const navigate = useNavigate();

  const refreshSessions = useCallback((noCache = false) => {
    setRefreshing(true);
    fetchSessions(10, 0, noCache)
      .then(setSessions)
      .catch(() => {})
      .finally(() => {
        setRefreshing(false);
        setClientReady(true);
      });
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => {
      refreshSessions();
    }, 0);

    const handleSessionsUpdated = () => {
      refreshSessions(true);
    };
    window.addEventListener(SESSIONS_UPDATED_EVENT, handleSessionsUpdated);

    return () => {
      clearTimeout(timer);
      window.removeEventListener(SESSIONS_UPDATED_EVENT, handleSessionsUpdated);
    };
  }, [refreshSessions]);

  const handleDelete = async (sessionId: string) => {
    try {
      await deleteSession(sessionId);
      refreshSessions(true);
      triggerSessionsUpdate();
      if (pathname === `/sessions/${sessionId}`) {
        navigate({ to: "/sessions" });
      }
    } catch (e) {
      console.error("Failed to delete session:", e);
    }
  };

  const handleRename = async (sessionId: string) => {
    if (!editingTitle.trim()) {
      setEditingId(null);
      return;
    }
    try {
      await renameSession(sessionId, editingTitle.trim());
      refreshSessions(true);
      triggerSessionsUpdate();
      setEditingId(null);
    } catch (e) {
      console.error("Failed to rename session:", e);
    }
  };

  if (!clientReady) {
    return (
      <div className="border-t pt-2 pb-4">
        <div className="flex items-center justify-between px-3 mb-1">
          <h3 className="text-xs font-semibold text-muted-foreground">
            最近对话
          </h3>
        </div>
        <div className="space-y-0.5 px-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-6 bg-muted/50 rounded-md animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  if (sessions.length === 0) return null;

  return (
    <div className="border-t pt-2 flex-1 min-h-0 flex flex-col">
      <div className="flex items-center justify-between px-3 mb-1 shrink-0">
        <h3 className="text-xs font-semibold text-muted-foreground">
          最近对话
        </h3>
        <Button
          variant="ghost"
          size="icon"
          className="h-5 w-5"
          onClick={() => refreshSessions(true)}
          disabled={refreshing}
          title="刷新会话列表"
        >
          <RefreshCw className={cn("h-3 w-3", refreshing && "animate-spin")} />
        </Button>
      </div>
      <div className="space-y-0.5 overflow-y-auto flex-1 min-h-0 px-1">
        {sessions.map((s) => {
          const href = `/sessions/${s.id}`;
          const active = pathname === href || pathname === `/possess/${s.id}`;
          const isEditing = editingId === s.id;

          return (
            <div key={s.id} className="group relative">
              {isEditing ? (
                <div className="flex items-center gap-1 px-2 py-1">
                  <Input
                    value={editingTitle}
                    onChange={(e) => setEditingTitle(e.target.value)}
                    className="h-5 text-xs px-1.5"
                    autoFocus
                    onKeyDown={(e) => {
                      if (e.nativeEvent.isComposing || e.keyCode === 229) return;
                      if (e.key === "Enter") handleRename(s.id);
                      if (e.key === "Escape") setEditingId(null);
                    }}
                  />
                  <Button size="icon" variant="ghost" className="h-5 w-5" onClick={() => handleRename(s.id)}>
                    <Check className="h-3 w-3" />
                  </Button>
                  <Button size="icon" variant="ghost" className="h-5 w-5" onClick={() => setEditingId(null)}>
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              ) : (
                <div className={cn(
                  "flex items-center gap-1.5 rounded-md pl-1.5 pr-1 py-1 text-xs transition-colors",
                  active ? "bg-primary/10" : "hover:bg-muted"
                )}>
                  {/* 模式色 + observation 计数 (替代原色点) */}
                  <span
                    className={cn(
                      "flex items-center gap-0.5 text-[10px] shrink-0 opacity-70",
                      MODE_COLORS_TEXT[s.mode as PossessionMode] || "text-gray-400"
                    )}
                    title={
                      s.observation_count > 0
                        ? `${modeLabel(s.mode)} · ${s.observation_count} 条压缩摘要`
                        : modeLabel(s.mode)
                    }
                  >
                    {s.observation_count > 0 && s.observation_count}
                  </span>
                  <a
                    href={`/sessions/${s.id}`}
                    onClick={(e) => {
                      e.preventDefault()
                      navigate({ to: "/sessions/$id", params: { id: s.id } })
                    }}
                    data-testid={`sidebar-session-${s.id}`}
                    className={cn(
                      "flex-1 min-w-0 truncate",
                      active
                        ? "text-primary font-medium"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    {s.title}
                  </a>
                  <span className="text-[10px] text-muted-foreground/50 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                    {modeLabel(s.mode)}
                  </span>
                  <div className="flex items-center shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-5 w-5"
                      title="重命名"
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setEditingTitle(s.title);
                        setEditingId(s.id);
                      }}
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                    <ConfirmButton
                      icon={<Trash2 className="h-3 w-3 text-red-500" />}
                      confirmText="确认删除"
                      title="删除会话"
                      size="icon"
                      className="h-5 w-5"
                      onConfirm={async () => {
                        await handleDelete(s.id);
                      }}
                    />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
