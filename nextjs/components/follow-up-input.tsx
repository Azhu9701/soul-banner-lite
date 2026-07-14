"use client";

import { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Send, Loader2, AlertCircle, CheckCircle2, StopCircle, ChevronDown, ChevronUp, Paperclip, X, FileText, ImageIcon, Globe } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { API_BASE, fetchSouls, type SoulListEntry } from "@/lib/api";
import { cn } from "@/lib/utils";
import { SynthesisSection } from "@/components/synthesis-section";
import { SoulResponseCard } from "@/components/soul-response-card";

const WS_HOST = API_BASE.replace("http://", "ws://").replace("/api/v1", "");
const FLUSH_INTERVAL_MS = 50;

const MAX_ATTACHMENTS = 3;
const MAX_ATTACHMENT_SIZE = 5 * 1024 * 1024;
const ALLOWED_ATTACHMENT_MIMES = [
  "image/png",
  "image/jpeg",
  "image/webp",
  "image/gif",
  "text/plain",
  "text/markdown",
  "application/pdf",
];

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

interface LocalMsg {
  role: "user" | "assistant";
  content: string;
  reasoningContent: string;
  id: string;
  streaming?: boolean;
  error?: string;
  /** 当通过推荐角色召唤时，记录角色名以便用角色卡片渲染 */
  soulName?: string;
}

export default function FollowUpInput({
  sessionId,
  trigger,
  sessionSouls,
}: {
  sessionId: string;
  /** When set, auto-fills the textarea and sends as follow-up with the named soul */
  trigger?: { question: string; soul?: string } | null;
  /** Souls already in session — shown first in @mention suggestions */
  sessionSouls?: string[];
}) {
  const navigate = useNavigate();
  const [followUp, setFollowUp] = useState(() => {
    if (typeof window === "undefined") return "";
    return localStorage.getItem(`followup-draft-${sessionId}`) || "";
  });
  const [sending, setSending] = useState(false);
  const [localMsgs, setLocalMsgs] = useState<LocalMsg[]>([]);
  const [error, setError] = useState("");
  const [expandedMsgId, setExpandedMsgId] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<File[]>([]);
  const [searchEnabled, setSearchEnabled] = useState(true);
  const [dragOver, setDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const contentRef = useRef("");
  const reasoningContentRef = useRef("");
  const currentMsgIdRef = useRef<string | null>(null);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const followUpReadyRef = useRef(false);
  const pendingFlushRef = useRef(false);
  const bottomRef = useRef<HTMLDivElement>(null);
  const lastScrollRef = useRef(0);
  const mountedRef = useRef(true);
  const sendingRef = useRef(false);

  // --- @mention state ---
  const [allSouls, setAllSouls] = useState<SoulListEntry[]>([]);
  const [mentionQuery, setMentionQuery] = useState<string | null>(null);
  const [mentionIdx, setMentionIdx] = useState(0);
  const [mentionStart, setMentionStart] = useState(-1);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const mentionBoxRef = useRef<HTMLDivElement>(null);

  const sessionSoulSet = useMemo(() => new Set(sessionSouls ?? []), [sessionSouls]);

  // Load all soul names once
  useEffect(() => {
    fetchSouls().then(setAllSouls).catch(() => {});
  }, []);

  const mentionSuggestions = useMemo(() => {
    if (mentionQuery === null) return [];
    const q = mentionQuery.toLowerCase();
    return allSouls
      .filter((s) => s.name.toLowerCase().includes(q))
      .sort((a, b) => {
        const aIn = sessionSoulSet.has(a.name) ? 0 : 1;
        const bIn = sessionSoulSet.has(b.name) ? 0 : 1;
        if (aIn !== bIn) return aIn - bIn;
        return a.name.localeCompare(b.name);
      });
  }, [mentionQuery, allSouls, sessionSoulSet]);

  // Reset highlight index when suggestions change
  useEffect(() => {
    setMentionIdx(0);
  }, [mentionQuery]);

  // Detect @mention pattern from cursor position
  const detectMention = useCallback((text: string, cursorPos: number) => {
    // Look backwards from cursor for @
    let atPos = -1;
    for (let i = cursorPos - 1; i >= 0; i--) {
      if (text[i] === '@') { atPos = i; break; }
      if (text[i] === ' ' || text[i] === '\n') break; // stop at whitespace
    }
    if (atPos === -1) {
      setMentionQuery(null);
      setMentionStart(-1);
      return;
    }
    // @ must be at start or preceded by whitespace
    if (atPos > 0 && text[atPos - 1] !== ' ' && text[atPos - 1] !== '\n') {
      setMentionQuery(null);
      setMentionStart(-1);
      return;
    }
    const query = text.slice(atPos + 1, cursorPos);
    setMentionStart(atPos);
    setMentionQuery(query);
  }, []);

  const insertMention = useCallback((soulName: string) => {
    const ta = textareaRef.current;
    if (!ta || mentionStart === -1) return;
    const text = followUp;
    const cursorPos = ta.selectionStart ?? text.length;
    const before = text.slice(0, mentionStart);
    const after = text.slice(cursorPos);
    const newText = `${before}@${soulName} ${after}`;
    setFollowUp(newText);
    setMentionQuery(null);
    setMentionStart(-1);
    // Place cursor after inserted soul name
    requestAnimationFrame(() => {
      const pos = before.length + soulName.length + 2; // +2 for @ and space
      ta.selectionStart = ta.selectionEnd = pos;
      ta.focus();
    });
  }, [followUp, mentionStart]);

  // Extract @mentioned soul from text (last @mention before sending)
  const extractMentionedSoul = useCallback((text: string): { soul: string | undefined; cleanText: string } => {
    const match = text.match(/@([^@\s]+)\s*/);
    if (!match) return { soul: undefined, cleanText: text };
    const soulName = match[1];
    // Check if this soul exists (exact or substring match)
    const found = allSouls.find((s) =>
      s.name === soulName || s.name.includes(soulName) || soulName.includes(s.name)
    );
    if (!found) return { soul: undefined, cleanText: text };
    // Remove the @mention from text
    const cleanText = text.replace(/@([^@\s]+)\s*/, '').trim();
    return { soul: found.name, cleanText };
  }, [allSouls]);

  const log = (_msg: string, ..._args: unknown[]) => {
    // debug logging disabled
  };

  const scheduleFlush = useCallback(() => {
    if (pendingFlushRef.current || !mountedRef.current) return;
    pendingFlushRef.current = true;
    // Use setTimeout(0) instead of rAF: rAF pauses in background tabs,
    // which would freeze the streaming UI entirely.
    flushTimerRef.current = setTimeout(() => {
      flushTimerRef.current = null;
      pendingFlushRef.current = false;
      if (!mountedRef.current || !currentMsgIdRef.current) return;
      setLocalMsgs((prev) =>
        prev.map((msg) =>
          msg.id === currentMsgIdRef.current
            ? {
                ...msg,
                content: contentRef.current,
                reasoningContent: reasoningContentRef.current,
                streaming: true,
              }
            : msg
        )
      );
      const now = Date.now();
      if (now - lastScrollRef.current > 500) {
        lastScrollRef.current = now;
        bottomRef.current?.scrollIntoView({ behavior: "instant", block: "end" });
      }
    }, 0);
  }, []);
  const flushImmediate = useCallback(() => {
    pendingFlushRef.current = false;
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    if (!mountedRef.current || !currentMsgIdRef.current) return;
    setLocalMsgs((prev) =>
      prev.map((msg) =>
        msg.id === currentMsgIdRef.current
          ? {
              ...msg,
              content: contentRef.current,
              reasoningContent: reasoningContentRef.current,
              streaming: false,
            }
          : msg
      )
    );
  }, []);

  const cleanup = useCallback(() => {
    log("Cleaning up...");
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
    }
  }, []);

  const stopGeneration = useCallback(() => {
    log("Stopping generation...");
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    sendingRef.current = false;
    setSending(false);
    flushImmediate();
    cleanup();
  }, [cleanup, flushImmediate]);

  // Core send logic shared by manual follow-up and trigger-based summon
  const _send = useCallback(async (question: string, soul?: string) => {
    // Close previous WS synchronously to avoid race:
    // old unsubscribe (system_channel.clear()) must finish before new subscribe
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
      wsRef.current = null;
    }
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }

    setMsgHistoryIdx(-1);
    setFollowUp("");
    setAttachments([]);
    setSending(true);
    sendingRef.current = true;
    setError("");
    contentRef.current = "";
    reasoningContentRef.current = "";
    followUpReadyRef.current = false;

    const qId = `q-${Date.now()}`;
    const aId = `a-${Date.now()}`;
    currentMsgIdRef.current = aId;

    setLocalMsgs((prev) => [
      ...prev,
      { role: "user", content: question, reasoningContent: "", id: qId },
      {
        role: "assistant",
        content: "",
        reasoningContent: "",
        id: aId,
        streaming: true,
        soulName: soul || undefined,
      },
    ]);

    const wsUrl = `${WS_HOST}/ws/possess/${sessionId}/main`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      log("Sending follow-up, question:", question, "soul:", soul);
      const formData = new FormData();
      formData.append("question", question);
      if (soul) formData.append("soul", soul);
      formData.append("search", searchEnabled ? "true" : "false");
      fetch(`${API_BASE}/possess/${sessionId}/follow-up`, {
        method: "POST",
        body: formData,
      }).then(() => {
        followUpReadyRef.current = true;
      }).catch((err) => {
        log("Error sending follow-up request:", err);
        setError(`发送失败: ${err.message}`);
        sendingRef.current = false;
        setSending(false);
        cleanup();
      });
    };

    setupWsHandlers(ws);
  }, [sessionId, scheduleFlush, flushImmediate, cleanup]);

  // WS event handlers — shared by manual and trigger sends.
  // Must be a regular function (not useCallback) — hoisted so _send can call it.
  function setupWsHandlers(ws: WebSocket) {
    ws.onmessage = (event) => {
      log("Received WebSocket message:", event.data);
      try {
        const msg = JSON.parse(event.data);
        if (msg.event_type === "synthesis_chunk" || msg.event_type === "soul_token") {
          if (msg.reasoning_content) {
            reasoningContentRef.current += msg.reasoning_content;
          }
          if (msg.payload) {
            contentRef.current += msg.payload;
          }
          scheduleFlush();
        } else if (msg.event_type === "synthesis_done" || msg.event_type === "soul_done") {
          // Only accept done events after the follow-up HTTP POST has been sent.
          if (!followUpReadyRef.current) return;
          flushImmediate();
          // 流结束后立即清空 ref，防止内容在 ref 中无限累加导致 OOM
          contentRef.current = "";
          reasoningContentRef.current = "";
          setSending(false);
          cleanup();
          // refresh handled by React state
        } else if (msg.event_type === "error") {
          setError(msg.payload);
          flushImmediate();
          setSending(false);
          cleanup();
        }
      } catch {}
    };

    ws.onerror = () => {
      if (sendingRef.current) {
        flushImmediate();
        sendingRef.current = false;
        setSending(false);
      }
    };

    ws.onclose = () => {
      if (sendingRef.current) {
        sendingRef.current = false;
        flushImmediate();
        setSending(false);
      }
    };
  }

  // Manual follow-up: user types + hits Enter
  const onFollowUp = useCallback(async () => {
    if ((!followUp.trim() && attachments.length === 0) || sending) return;
    const { soul, cleanText } = extractMentionedSoul(followUp.trim());
    const q = cleanText || followUp.trim();
    if (q) {
      setMsgHistory((prev) => {
        const next = [q, ...prev.filter((m) => m !== q)].slice(0, 50);
        return next;
      });
    }
    setMentionQuery(null);
    setMentionStart(-1);
    await _send(q, soul);
  }, [followUp, attachments, sending, _send, extractMentionedSoul]);

  // Trigger-based summon: called when onSummon sets trigger prop (soul recommendation card)
  const triggeredRef = useRef<{ question: string; soul?: string } | null>(null);
  useEffect(() => {
    if (trigger && (trigger.question !== triggeredRef.current?.question || trigger.soul !== triggeredRef.current?.soul)) {
      triggeredRef.current = trigger;
      _send(trigger.question, trigger.soul || undefined);
    }
  }, [trigger, _send]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      cleanup();
    };
  }, [cleanup]);

  useEffect(() => {
    if (followUp) {
      localStorage.setItem(`followup-draft-${sessionId}`, followUp);
    } else {
      localStorage.removeItem(`followup-draft-${sessionId}`);
    }
  }, [followUp, sessionId]);

  const [msgHistory, setMsgHistory] = useState<string[]>([]);
  const [msgHistoryIdx, setMsgHistoryIdx] = useState(-1);

  const dismissError = () => setError("");

  const validateAndAddFiles = useCallback((files: File[]) => {
    if (files.length === 0) return;
    const rejected: string[] = [];
    const accepted: File[] = [];
    for (const f of files) {
      if (attachments.length + accepted.length >= MAX_ATTACHMENTS) {
        rejected.push(`${f.name}（超过 ${MAX_ATTACHMENTS} 个上限）`);
        continue;
      }
      if (f.size > MAX_ATTACHMENT_SIZE) {
        rejected.push(`${f.name}（${formatBytes(f.size)} 超 5MB）`);
        continue;
      }
      const mime = f.type || "application/octet-stream";
      const isAllowed = ALLOWED_ATTACHMENT_MIMES.includes(mime) || /\.(png|jpe?g|webp|gif|txt|md|pdf)$/i.test(f.name);
      if (!isAllowed) {
        rejected.push(`${f.name}（类型 ${mime} 不支持）`);
        continue;
      }
      accepted.push(f);
    }
    if (accepted.length > 0) {
      setAttachments((prev) => [...prev, ...accepted]);
    }
    if (rejected.length > 0) {
      setError(`部分文件被拒绝：${rejected.join("；")}`);
    }
  }, [attachments.length]);

  const removeAttachment = useCallback((index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const onFileInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files) {
      validateAndAddFiles(Array.from(e.target.files));
      e.target.value = "";
    }
  }, [validateAndAddFiles]);

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!sending) setDragOver(true);
  }, [sending]);

  const onDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
  }, []);

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
    if (sending) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      validateAndAddFiles(Array.from(e.dataTransfer.files));
    }
  }, [sending, validateAndAddFiles]);

  return (
    <div className="space-y-4">
      {error && (
        <div className="rounded-lg border border-red-200 bg-red-50 dark:bg-red-950/30 p-4 flex items-start gap-3">
          <AlertCircle className="h-5 w-5 text-red-500 shrink-0 mt-0.5" />
          <div className="flex-1">
            <p className="text-red-600 dark:text-red-400 font-medium">发生错误</p>
            <p className="text-red-500 dark:text-red-300 text-sm mt-1">{error}</p>
          </div>
          <Button variant="ghost" size="sm" onClick={dismissError} className="text-red-600 hover:text-red-700 hover:bg-red-100 dark:hover:bg-red-900/30">
            关闭
          </Button>
        </div>
      )}

      {localMsgs.length > 0 && (
        <div className="space-y-4 pb-80">
          {localMsgs.map((msg) => (
            <div key={msg.id} className="space-y-2">
              {msg.role === "user" ? (
                <div className="rounded-xl p-4 text-sm bg-primary/5 ml-8 border border-primary/10">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-xs font-semibold text-muted-foreground flex items-center gap-1.5">
                      <CheckCircle2 className="h-3 w-3 text-green-500" />
                      用户
                    </span>
                  </div>
                  <p className="text-sm leading-relaxed whitespace-pre-wrap">{msg.content}</p>
                </div>
              ) : msg.soulName ? (
                <SoulResponseCard
                  name={msg.soulName}
                  content={msg.content}
                  isStreaming={msg.streaming}
                />
              ) : (
                <SynthesisSection
                  streaming={msg.streaming}
                  messages={[{ id: msg.id, content: msg.content, created_at: new Date().toISOString() }]}
                />
              )}
            </div>
          ))}
          <div ref={bottomRef} />
        </div>
      )}

      <div
        className={cn(
          "sticky bottom-0 bg-background/95 backdrop-blur-sm border-t pt-4 pb-2 mt-4 transition-colors",
          dragOver && "ring-2 ring-primary/40 bg-primary/5 border-primary/30"
        )}
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onDrop={onDrop}
      >
        {dragOver && (
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none rounded-lg">
            <div className="bg-background/90 px-4 py-2 rounded-lg border-2 border-dashed border-primary/50 text-sm font-medium text-primary">
              松开以添加附件（最多 {MAX_ATTACHMENTS} 个，单个 ≤5MB）
            </div>
          </div>
        )}

        {attachments.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-2">
            {attachments.map((f, i) => {
              const isImage = f.type.startsWith("image/");
              return (
                <div
                  key={`${f.name}-${i}`}
                  className="flex items-center gap-1.5 bg-muted/60 border border-border rounded-md pl-2 pr-1 py-1 text-xs"
                  data-testid={`attachment-${i}`}
                >
                  {isImage ? (
                    <ImageIcon className="h-3.5 w-3.5 text-blue-500 shrink-0" />
                  ) : (
                    <FileText className="h-3.5 w-3.5 text-purple-500 shrink-0" />
                  )}
                  <span className="max-w-[140px] truncate" title={f.name}>{f.name}</span>
                  <span className="text-muted-foreground">{formatBytes(f.size)}</span>
                  <button
                    type="button"
                    onClick={() => removeAttachment(i)}
                    disabled={sending}
                    className="ml-0.5 p-0.5 rounded hover:bg-muted-foreground/20 disabled:opacity-50"
                    aria-label={`移除 ${f.name}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              );
            })}
          </div>
        )}

        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept=".png,.jpg,.jpeg,.webp,.gif,.txt,.md,.pdf,image/*,text/plain,text/markdown,application/pdf"
          className="hidden"
          onChange={onFileInputChange}
        />

        <div className="flex gap-2 items-end">
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={() => fileInputRef.current?.click()}
            disabled={sending || attachments.length >= MAX_ATTACHMENTS}
            className="h-[66px] w-11 shrink-0"
            title={`添加附件（${attachments.length}/${MAX_ATTACHMENTS}）`}
            data-testid="attach-btn"
          >
            <Paperclip className="h-4 w-4" />
          </Button>
          <Button
            type="button"
            variant={searchEnabled ? "default" : "outline"}
            size="icon"
            onClick={() => setSearchEnabled(!searchEnabled)}
            disabled={sending}
            className="h-[66px] w-11 shrink-0"
            title={searchEnabled ? "联网搜索已开启" : "联网搜索已关闭"}
            data-testid="search-toggle-btn"
          >
            <Globe className={`h-4 w-4 ${searchEnabled ? "" : "opacity-40"}`} />
          </Button>

          <div className="flex-1 relative">
            {mentionQuery !== null && mentionSuggestions.length > 0 && (
              <div
                ref={mentionBoxRef}
                className="absolute bottom-full left-0 mb-1 w-72 max-h-64 overflow-y-auto rounded-lg border bg-white dark:bg-gray-900 shadow-xl z-50"
              >
                {mentionSuggestions.map((s, i) => (
                  <button
                    key={s.name}
                    type="button"
                    className={cn(
                      "w-full text-left px-3 py-2 text-sm flex items-center gap-2 transition-colors",
                      i === mentionIdx ? "bg-accent text-accent-foreground" : "hover:bg-muted"
                    )}
                    onMouseDown={(e) => { e.preventDefault(); insertMention(s.name); }}
                    onMouseEnter={() => setMentionIdx(i)}
                  >
                    <span className="font-medium truncate">{s.name}</span>
                    {sessionSoulSet.has(s.name) && (
                      <span className="text-[10px] px-1 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-400 shrink-0">已参会</span>
                    )}
                    <span className="text-xs text-muted-foreground ml-auto truncate max-w-[100px]">{s.field}</span>
                  </button>
                ))}
              </div>
            )}
            <Textarea
              ref={textareaRef}
              placeholder={attachments.length > 0 ? "可加附注，或直接发送附件（Shift+Enter 换行）" : "输入追问... (@角色名 召唤特定角色，Enter 发送，Shift+Enter 换行)"}
              value={followUp}
              onChange={(e) => {
                setFollowUp(e.target.value);
                setMsgHistoryIdx(-1);
                detectMention(e.target.value, e.target.selectionStart ?? e.target.value.length);
              }}
              rows={2}
              className="flex-1 resize-none transition-all focus:ring-2 focus:ring-primary/20"
              onKeyDown={(e) => {
                // IME 组字中（中文/日文等输入法候选阶段），Enter 用于选词，不应提交
                if (e.nativeEvent.isComposing || e.keyCode === 229) {
                  return;
                }
                // @mention navigation
                if (mentionQuery !== null && mentionSuggestions.length > 0) {
                  if (e.key === "ArrowDown") {
                    e.preventDefault();
                    setMentionIdx((prev) => Math.min(prev + 1, mentionSuggestions.length - 1));
                    return;
                  }
                  if (e.key === "ArrowUp") {
                    e.preventDefault();
                    setMentionIdx((prev) => Math.max(prev - 1, 0));
                    return;
                  }
                  if (e.key === "Enter" || e.key === "Tab") {
                    e.preventDefault();
                    insertMention(mentionSuggestions[mentionIdx].name);
                    return;
                  }
                  if (e.key === "Escape") {
                    e.preventDefault();
                    setMentionQuery(null);
                    setMentionStart(-1);
                    return;
                  }
                }
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onFollowUp();
                  return;
                }
                if (e.key === "ArrowUp" && msgHistory.length > 0 && mentionQuery === null) {
                const textarea = e.currentTarget as HTMLTextAreaElement;
                if (textarea.selectionStart !== textarea.selectionEnd) return;
                if (msgHistoryIdx === -1) {
                  const nextIdx = 0;
                  setMsgHistoryIdx(nextIdx);
                  setFollowUp(msgHistory[nextIdx]);
                } else if (msgHistoryIdx < msgHistory.length - 1) {
                  const nextIdx = msgHistoryIdx + 1;
                  setMsgHistoryIdx(nextIdx);
                  setFollowUp(msgHistory[nextIdx]);
                }
                e.preventDefault();
                return;
              }
              if (e.key === "ArrowDown" && msgHistoryIdx >= 0) {
                if (msgHistoryIdx === 0) {
                  setMsgHistoryIdx(-1);
                  setFollowUp("");
                } else {
                  const nextIdx = msgHistoryIdx - 1;
                  setMsgHistoryIdx(nextIdx);
                  setFollowUp(msgHistory[nextIdx]);
                }
                e.preventDefault();
              }
            }}
            disabled={sending}
            data-testid="follow-up-input"
          />
          </div>
          {sending ? (
            <Button
              onClick={stopGeneration}
              data-testid="stop-btn"
              className="h-[66px] px-4 bg-red-500 hover:bg-red-600"
            >
              <StopCircle className="h-5 w-5" />
            </Button>
          ) : (
            <Button
              onClick={onFollowUp}
              disabled={!followUp.trim() && attachments.length === 0}
              data-testid="follow-up-btn"
              className="h-[66px] px-4"
            >
              <Send className="h-5 w-5" />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
