"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import { SESSIONS_UPDATED_EVENT } from "@/components/sidebar-sessions";

const API_BASE = import.meta.env.VITE_API_URL || "/api/v1";

export interface ProcessStep {
  event: string;
  message: string;
  soulName: string | null;
  timestamp: Date;
}

export interface LogEntry {
  timestamp: Date;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
}

export interface WsEvent {
  event_type: string;
  payload: string;
  soul_name: string | null;
  seq: number;
}

export interface SoulMessage {
  soulName: string;
  content: string;
  isStreaming: boolean;
  error: string | null;
  ismismCode?: string;
}

export interface SoulCostBreakdown {
  soul_name: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  provider?: string;
}

export interface CostInfo {
  llm_calls: number;
  tokens_used: number;
  estimated_cost?: string;
  per_soul?: SoulCostBreakdown[];
}

export interface CollisionEvent {
  from: string;
  to: string;
  content: string;
  injected?: boolean;
}

export interface ToolCallEvent {
  toolCallId: string;
  toolName: string;
  arguments: string;
  soulName: string;
  status: 'calling' | 'done';
  result?: string;
}

export interface SoulRecommendation {
  name: string;
  rationale: string;
  subtask?: string;
}

export interface DigestReadyEvent {
  summary: string;
  observation_count: number;
}

const MAX_RETRIES = 3;
const FLUSH_INTERVAL_MS = 50; // batch state updates at ~20fps
const MAX_SOUL_CONTENT_BYTES = 80_000; // 80KB per soul — 远超正常 LLM 输出，防止异常流式洪水
const MAX_COST_PER_SOUL_ENTRIES = 20;   // 防止 cost 事件无限 push

const EVENT_LABEL: Record<string, string> = {
  session_started: "SessionStarted",
  entry_classified: "EntryClassified",
  soul_started: "SoulStarted",
  synthesis_started: "SynthesisStarted",
  process_step: "SearchComplete",
};

export function useWebSocket(sessionId: string) {
  const wsRef = useRef<WebSocket | null>(null);
  const retryRef = useRef(0);
  const hasConnectedBeforeRef = useRef(false);
  const bufferRef = useRef<Record<string, SoulMessage>>({});
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingFlushRef = useRef(false);
  const mountedRef = useRef(true);
  const noEventsTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // forceTick: counter that React sees — incrementing it triggers a re-render
  // This is the ONLY React state for streaming content, avoiding per-token setState overhead
  const [tick, setTick] = useState(0);
  // Snapshot of buffer for React consumption — only updated on flush
  const [messages, setMessages] = useState<Record<string, SoulMessage>>({});

  const [synthesis, setSynthesis] = useState("");
  const synthesisRef = useRef("");
  const [status, setStatus] = useState<"connecting" | "streaming" | "done" | "error">("connecting");
  const [error, setError] = useState<string | null>(null);
  const [processSteps, setProcessSteps] = useState<ProcessStep[]>([]);
  const [cost, setCost] = useState<CostInfo | null>(null);
  const costPerSoulRef = useRef<SoulCostBreakdown[]>([]);
  const [collisions, setCollisions] = useState<CollisionEvent[]>([]);
  const [toolCalls, setToolCalls] = useState<ToolCallEvent[]>([]);
  const [soulRecommendations, setSoulRecommendations] = useState<SoulRecommendation[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [digestReady, setDigestReady] = useState<DigestReadyEvent | null>(null);

  // Flush buffer to React state at controlled intervals
  const scheduleFlush = useCallback(() => {
    if (pendingFlushRef.current || !mountedRef.current) return;
    pendingFlushRef.current = true;
    flushTimerRef.current = setTimeout(() => {
      pendingFlushRef.current = false;
      if (!mountedRef.current) return;
      // Snapshot buffer content into React state
      setMessages({ ...bufferRef.current });
      setSynthesis(synthesisRef.current);
      setTick((t) => t + 1);
    }, FLUSH_INTERVAL_MS);
  }, []);

  // Immediate flush (for done/error events that shouldn't wait)
  const flushImmediate = useCallback(() => {
    if (flushTimerRef.current) {
      clearTimeout(flushTimerRef.current);
      pendingFlushRef.current = false;
    }
    if (!mountedRef.current) return;
    setMessages({ ...bufferRef.current });
    setSynthesis(synthesisRef.current);
    setTick((t) => t + 1);
  }, []);

  const MAX_PROCESS_STEPS = 100;
  const MAX_LOGS = 200;
  const MAX_COLLISIONS = 50;
  const MAX_TOOL_CALLS = 50;

  const addStep = (event: string, message: string, soulName: string | null) => {
    const label = EVENT_LABEL[event] || event;
    setProcessSteps((prev) => {
      const next = [...prev, { event: label, message, soulName, timestamp: new Date() }];
      return next.length > MAX_PROCESS_STEPS ? next.slice(-MAX_PROCESS_STEPS) : next;
    });
  };

  const addLog = (message: string, type: LogEntry['type'] = 'info') => {
    setLogs((prev) => {
      const next = [...prev, { timestamp: new Date(), message, type }];
      return next.length > MAX_LOGS ? next.slice(-MAX_LOGS) : next;
    });
  };

  const connect = useCallback(() => {
    // 清理待执行的重连定时器，防止新旧 connect 交叠导致定时器累积
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    // Close previous connection without triggering its onclose retry
    if (wsRef.current) {
      wsRef.current.onclose = null;
      wsRef.current.close();
    }
    const apiBase = import.meta.env.VITE_API_URL || "/api/v1";
    const wsHost = apiBase.replace("http://", "ws://").replace("https://", "wss://").replace("/api/v1", "");
    const url = `${wsHost}/ws/possess/${sessionId}/main`;
    const ws = new WebSocket(url);
    wsRef.current = ws;
    mountedRef.current = true;
    setStatus("connecting");

    // ── Always clear streaming state on (re)connect; process steps replays from server buffer ──
    setProcessSteps([]);
    bufferRef.current = {};
    synthesisRef.current = "";
    setMessages({});
    setSynthesis("");
    setCost(null);
    costPerSoulRef.current = [];
    setCollisions([]);
    setToolCalls([]);
    setSoulRecommendations([]);
    setDigestReady(null);
    if (!hasConnectedBeforeRef.current) {
      setLogs([]);
    }

    // Reset idle timer — fires if no events arrive for 8s (session already complete)
    const resetIdleTimer = () => {
      if (noEventsTimeoutRef.current) clearTimeout(noEventsTimeoutRef.current);
      noEventsTimeoutRef.current = setTimeout(() => {
        if (!mountedRef.current) return;
        setStatus("done");
        addLog("会话可能已结束 — 尝试从 API 恢复", 'info');
      }, 8000);
    };

    ws.onopen = () => {
      retryRef.current = 0;
      hasConnectedBeforeRef.current = true;
      setStatus("streaming");
      setError(null);
      addLog("WebSocket 连接已建立", 'success');
      resetIdleTimer();
    };

    ws.onmessage = (e) => {
      const event: WsEvent = JSON.parse(e.data);
      resetIdleTimer();

      switch (event.event_type) {
        // Process events — update React state directly (low frequency)
        case "session_started":
          addStep(event.event_type, event.payload, event.soul_name);
          addLog(`会话已启动: ${event.payload}`, 'success');
          break;
        case "entry_classified":
          addStep(event.event_type, event.payload, event.soul_name);
          addLog(`入口分流完成: ${event.payload}`, 'info');
          break;
        case "soul_started":
          addStep(event.event_type, event.payload, event.soul_name);
          addLog(`正在召唤魂: ${event.soul_name || event.payload}`, 'info');
          break;
        case "synthesis_started":
          addStep(event.event_type, event.payload, event.soul_name);
          addLog(`开始综合分析...`, 'info');
          break;

        case "process_step":
          addStep(event.event_type, event.payload, event.soul_name);
          addLog(`📡 ${event.payload}`, 'info');
          break;

        // Soul streaming — write to ref (synchronous), schedule throttled flush
        case "soul_token": {
          const soulName = event.soul_name;
          if (soulName) {
            if (!bufferRef.current[soulName]) {
              bufferRef.current[soulName] = {
                soulName,
                content: "",
                isStreaming: true,
                error: null,
              };
            } else {
              const current = bufferRef.current[soulName];
              // 字节上限保护：超过 80KB 后不再累加，防止异常流式洪水导致 OOM
              if (current.content.length < MAX_SOUL_CONTENT_BYTES) {
                bufferRef.current[soulName] = {
                  ...current,
                  content: current.content + event.payload,
                };
              }
            }
            scheduleFlush();
          }
          break;
        }

        case "soul_done":
          if (event.soul_name && bufferRef.current[event.soul_name]) {
            bufferRef.current[event.soul_name] = {
              ...bufferRef.current[event.soul_name],
              isStreaming: false,
            };
            flushImmediate();
          }
          addLog(`魂回应完成: ${event.soul_name}`, 'success');
          break;

        case "soul_error":
          if (event.soul_name && bufferRef.current[event.soul_name]) {
            bufferRef.current[event.soul_name] = {
              ...bufferRef.current[event.soul_name],
              error: event.payload,
              isStreaming: false,
            };
            flushImmediate();
          }
          addLog(`魂出错: ${event.soul_name} - ${event.payload}`, 'error');
          break;

        case "synthesis_chunk":
          synthesisRef.current += event.payload;
          scheduleFlush();
          break;

        case "synthesis_done":
          flushImmediate();
          synthesisRef.current = "";  // 流结束立即清空 ref，防止无限累加
          addLog(`综合分析完成`, 'success');
          break;

        case "collision":
          try {
            const c = JSON.parse(event.payload) as CollisionEvent;
            setCollisions((prev) => {
              const next = [...prev, c];
              return next.length > MAX_COLLISIONS ? next.slice(-MAX_COLLISIONS) : next;
            });
          } catch {}
          break;

        case "cost":
          try {
            const payload = JSON.parse(event.payload);
            if (payload.soul_name) {
              costPerSoulRef.current = [...costPerSoulRef.current, payload as SoulCostBreakdown];
              if (costPerSoulRef.current.length > MAX_COST_PER_SOUL_ENTRIES) {
                costPerSoulRef.current = costPerSoulRef.current.slice(-MAX_COST_PER_SOUL_ENTRIES);
              }
            } else {
              const perSoul = payload.per_soul || costPerSoulRef.current;
              setCost({ llm_calls: payload.llm_calls || 0, tokens_used: payload.tokens_used || 0, per_soul: perSoul });
            }
          } catch {}
          break;

        case "tool_call_started":
          try {
            const tc = JSON.parse(event.payload) as {
              tool_call_id: string;
              tool_name: string;
              arguments: string;
              soul_name: string;
            };
            setToolCalls((prev) => {
              const next = [
                ...prev,
                {
                  toolCallId: tc.tool_call_id,
                  toolName: tc.tool_name,
                  arguments: tc.arguments,
                  soulName: tc.soul_name,
                  status: 'calling' as const,
                },
              ];
              return next.length > MAX_TOOL_CALLS ? next.slice(-MAX_TOOL_CALLS) : next;
            });
            addLog(`${tc.soul_name} 调用工具: ${tc.tool_name}`, 'info');
          } catch {}
          break;

        case "tool_result":
          try {
            const tr = JSON.parse(event.payload) as {
              tool_call_id: string;
              tool_name: string;
              result: string;
              soul_name: string;
            };
            setToolCalls((prev) =>
              prev.map((tc) =>
                tc.toolCallId === tr.tool_call_id
                  ? { ...tc, status: 'done' as const, result: tr.result }
                  : tc
              )
            );
            addLog(`${tr.soul_name} 工具 ${tr.tool_name} 完成`, 'success');
          } catch {}
          break;

        case "soul_recommendations":
          try {
            const data = JSON.parse(event.payload);
            const recs: SoulRecommendation[] = data.recommendations || [];
            if (recs.length > 0) {
              setSoulRecommendations(recs);
              addLog(`综合官推荐补充魂: ${recs.map((r: SoulRecommendation) => r.name).join("、")}`, 'info');
            }
          } catch {}
          break;

        case "observations_ready":
          try {
            const digest = JSON.parse(event.payload) as DigestReadyEvent;
            setDigestReady(digest);
            addLog(`📝 记忆压缩完成: ${digest.observation_count} 条 observation`, 'success');
            // 通知 sidebar 和 timeline 刷新角标
            if (typeof window !== "undefined") {
              window.dispatchEvent(new CustomEvent(SESSIONS_UPDATED_EVENT));
            }
          } catch {}
          break;

        case "annotations_ready":
          try {
            const anns = JSON.parse(event.payload) as unknown[];
            addLog(`✦ 魂间互批已生成: ${anns.length} 条 marginalia`, 'success');
            if (typeof window !== "undefined") {
              window.dispatchEvent(new CustomEvent(SESSIONS_UPDATED_EVENT));
            }
          } catch {}
          break;

        case "done":
        case "SessionComplete":
          flushImmediate();
          synthesisRef.current = "";  // 防止 ref 中残留完整会话内容
          setStatus("done");
          addLog(`会话完成`, 'success');
          break;
      }
    };

    ws.onclose = () => {
      if (!mountedRef.current) return;
      if (retryRef.current < MAX_RETRIES) {
        // 清理旧的重连定时器，防止多次 connect 导致定时器累积泄漏
        if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
        reconnectTimerRef.current = setTimeout(connect, Math.pow(2, retryRef.current) * 1000);
        retryRef.current++;
      } else {
        setStatus("error");
        setError("连接已断开，请稍后重试");
      }
    };

    ws.onerror = () => ws.close();
  }, [sessionId, scheduleFlush, flushImmediate]);

  // Recover session content from REST API when WebSocket can't replay stream chunks
  const recoverFromApi = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/sessions/${sessionId}`);
      if (!res.ok) return;
      const detail = await res.json();
      const msgs = detail.messages || [];

      // Group soul messages by name
      const soulContent: Record<string, string> = {};
      let synthContent = "";
      const recoveredMessages: Record<string, SoulMessage> = {};

      for (const m of msgs) {
        if (m.role === "synthesis") {
          synthContent += (synthContent ? "\n\n" : "") + m.content;
        } else if ((m.role === "soul" || m.role === "assistant") && m.soul_name) {
          soulContent[m.soul_name] = (soulContent[m.soul_name] || "") + m.content;
        }
      }

      for (const [name, content] of Object.entries(soulContent)) {
        recoveredMessages[name] = {
          soulName: name,
          content,
          isStreaming: false,
          error: null,
        };
      }

      if (!mountedRef.current) return;

      if (Object.keys(recoveredMessages).length > 0) {
        bufferRef.current = recoveredMessages;
        setMessages({ ...recoveredMessages });
      }
      if (synthContent) {
        synthesisRef.current = synthContent;
        setSynthesis(synthContent);
      }
      if (Object.keys(recoveredMessages).length > 0 || synthContent) {
        setTick((t) => t + 1);
      }
    } catch {}
  }, [sessionId]);

  useEffect(() => {
    connect();
    return () => {
      mountedRef.current = false;
      if (flushTimerRef.current) clearTimeout(flushTimerRef.current);
      if (noEventsTimeoutRef.current) clearTimeout(noEventsTimeoutRef.current);
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
    };
  }, [connect]);

  // Trigger API recovery when session completes but content is missing
  const hasContent = Object.keys(messages).length > 0 || synthesis.length > 0;
  useEffect(() => {
    if (status === "done" && !hasContent) {
      recoverFromApi();
    }
  }, [status, hasContent, recoverFromApi]);

  // Immediate recovery when WebSocket connects but no stream events (completed session)
  useEffect(() => {
    if (status === "streaming" && !hasContent) {
      const timer = setTimeout(() => {
        if (!mountedRef.current) return;
        const stillNoContent = Object.keys(bufferRef.current).length === 0 && synthesisRef.current.length === 0;
        if (stillNoContent) {
          recoverFromApi();
        }
      }, 1500);
      return () => clearTimeout(timer);
    }
  }, [status, hasContent, recoverFromApi]);

  return {
    messages,
    synthesis,
    status,
    error,
    processSteps,
    cost,
    collisions,
    toolCalls,
    soulRecommendations,
    logs,
    tick,
    digestReady,
    reconnect: connect,
  };
}
