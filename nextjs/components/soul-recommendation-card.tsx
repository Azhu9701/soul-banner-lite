"use client";

import { useState, useEffect, useRef, useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Loader2, CheckCircle, UserPlus, ArrowRight, Target, ChevronDown, ChevronUp } from "lucide-react";
import { autoCreateSoul, watchAutoCreate, fetchSouls } from "@/lib/api";
import type { SoulRecommendation } from "@/hooks/use-websocket";

interface SoulRecommendationCardProps {
  recommendations: SoulRecommendation[];
  /** Called when user clicks "直接加入合议" — parent sets up follow-up with named soul */
  onSummonSoul?: (soulName: string, subtask?: string) => void;
  /** Souls already in this session — skip recommending them */
  sessionSouls?: string[];
}

function RecommendationItem({
  rec,
  exists,
  onCreate,
  onSummon,
  loading,
  done,
  progressMsg,
}: {
  rec: SoulRecommendation;
  exists: boolean;
  onCreate: () => void;
  onSummon: () => void;
  loading: boolean;
  done: boolean;
  progressMsg?: string;
}) {
  const [expanded, setExpanded] = useState(false);
  const canSummon = exists || done;
  const summonLabel = exists ? "直接加入合议" : "用此子任务召唤";

  return (
    <div className="flex flex-col gap-2 p-3 rounded-lg border bg-background/50 hover:bg-muted/30 transition-colors">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex items-start gap-3 text-left w-full"
      >
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-sm">{rec.name}</span>
            {exists ? (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-400 font-medium">
                已在幡
              </span>
            ) : (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-400 font-medium">
                待炼化
              </span>
            )}
            <span className="text-[10px] text-muted-foreground ml-auto shrink-0">
              {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
            </span>
          </div>
          <p className={`text-xs text-muted-foreground mt-1 ${expanded ? "" : "line-clamp-2"}`}>
            {rec.rationale}
          </p>
        </div>
      </button>

      {expanded && rec.subtask && (
        <div className="ml-7 pl-2 border-l-2 border-amber-300/60 dark:border-amber-700/40 space-y-1">
          <div className="flex items-center gap-1.5 text-[10px] text-amber-600 dark:text-amber-400 uppercase tracking-wide font-medium">
            <Target className="h-3 w-3" />
            推荐子任务
          </div>
          <p className="text-xs text-foreground italic leading-relaxed">{`"${rec.subtask}"`}</p>
        </div>
      )}

      {loading && (
        <div className="ml-7 text-[11px] text-muted-foreground flex items-center gap-1.5">
          <Loader2 className="h-3 w-3 animate-spin text-amber-500" />
          <span>{progressMsg || "收集角色中…"}</span>
        </div>
      )}

      <div className="flex items-center gap-2 justify-end mt-1">
        {!exists && (
          <button
            onClick={onCreate}
            disabled={loading || done}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {done ? (
              <>
                <CheckCircle className="h-3 w-3" />
                已入幡
              </>
            ) : loading ? (
              <>
                <Loader2 className="h-3 w-3 animate-spin" />
                炼化中…
              </>
            ) : (
              <>
                <UserPlus className="h-3 w-3" />
                收集角色
              </>
            )}
          </button>
        )}
        <button
          onClick={onSummon}
          disabled={!canSummon}
          title={canSummon ? `${summonLabel}：${rec.name}` : "请先炼化再召唤"}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md border border-amber-300 dark:border-amber-700 text-amber-700 dark:text-amber-300 hover:bg-amber-50 dark:hover:bg-amber-950/30 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
        >
          <ArrowRight className="h-3 w-3" />
          {summonLabel}
        </button>
      </div>
    </div>
  );
}

export function SoulRecommendationCard({
  recommendations,
  onSummonSoul,
  sessionSouls,
}: SoulRecommendationCardProps) {
  const navigate = useNavigate();
  interface SoulTaskState { loading: boolean; done: boolean; error: string; progress: string; }
  const [taskState, setTaskState] = useState<Record<string, SoulTaskState>>({});
  const [existingSouls, setExistingSouls] = useState<Set<string>>(new Set());
  const abortRef = useRef<Map<string, () => void>>(new Map());

  const sessionSoulSet = useMemo(() => new Set(sessionSouls ?? []), [sessionSouls]);

  // Only filter out souls already participating in this session
  const filtered = useMemo(() => {
    if (sessionSoulSet.size === 0) return recommendations;
    return recommendations.filter((rec) => {
      const clean = rec.name.replace(/[（(][^）)]*[）)]/g, "").trim();
      return ![...sessionSoulSet].some(
        (s) => clean === s || rec.name.includes(s) || s.includes(rec.name)
      );
    });
  }, [recommendations, sessionSoulSet]);

  // Load existing soul names once so we know which recommendations are already 入幡
  useEffect(() => {
    fetchSouls()
      .then((souls) => setExistingSouls(new Set(souls.map((s) => s.name))))
      .catch(() => {});
  }, []);

  const updateTask = (name: string, patch: Partial<SoulTaskState>) => {
    setTaskState((prev) => {
      const cur = prev[name] || { loading: false, done: false, error: "", progress: "" };
      return { ...prev, [name]: { ...cur, ...patch } };
    });
  };

  const handleCreate = async (name: string) => {
    updateTask(name, { loading: true, error: "", progress: "正在启动收集角色任务…" });

    try {
      const accepted = await autoCreateSoul(name);
      updateTask(name, { progress: "正在搜索资料…" });

      const { abort } = watchAutoCreate(accepted.task_id, name, (evt) => {
        if (evt.phase === 'error') {
          updateTask(name, { loading: false, error: evt.message || "收集角色失败" });
          return;
        }
        if (evt.message) {
          updateTask(name, { progress: evt.message! });
        }
        if (evt.phase === 'done') {
          updateTask(name, { loading: false, done: true });
          setExistingSouls((prev) => new Set(prev).add(name));
        }
      });
      abortRef.current.set(name, abort);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : (typeof e === "string" ? e : "收集角色失败");
      updateTask(name, { loading: false, error: msg });
      console.error(`[soul-recommendation] autoCreateSoul(${name}) failed:`, e);
    }
  };

  const handleSummon = (name: string, subtask?: string, rationale?: string) => {
    // Strip parenthetical annotations so "葛兰西（Antonio Gramsci）" → "葛兰西"
    const clean = name.replace(/[（(][^）)]*[）)]/g, "").trim();
    const sumonTask = subtask || rationale || `请以你的视角回应该议题`;
    if (onSummonSoul) {
      onSummonSoul(clean, sumonTask);
    } else {
      // Fallback: navigate to possess page with pre-selected soul
      const params = new URLSearchParams();
      params.set("souls", clean);
      if (subtask) params.set("task", subtask);
      navigate({ to: `/possess?${params.toString()}` });
    }
  };

  return (
    <div className="mt-4 p-4 rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50/30 dark:bg-amber-950/20">
      <div className="flex items-center gap-2 mb-3">
        <h3 className="text-sm font-semibold text-amber-700 dark:text-amber-300">
          综合官推荐补充角色
        </h3>
        <span className="text-xs text-muted-foreground">
          {filtered.length} 个
          {recommendations.length !== filtered.length && `（已排除 ${recommendations.length - filtered.length} 个已出庭角色）`}
        </span>
      </div>
      {filtered.length === 0 ? (
        <p className="text-xs text-muted-foreground">综合官推荐补充的角色均已出庭。</p>
      ) : (
        <div className="space-y-2">
        {filtered.map((rec) => {
          // Synthesis may output "费曼（Richard Feynman）［或任何强经验…］"
          // while the system name is just "费曼". Match by containment.
          const exists = existingSouls.has(rec.name)
            || [...existingSouls].some((s) => rec.name.includes(s) || s.includes(rec.name));
          return (
            <div key={rec.name}>
              <RecommendationItem
                rec={rec}
                exists={exists}
                onCreate={() => handleCreate(rec.name)}
                onSummon={() => handleSummon(rec.name, rec.subtask, rec.rationale)}
                loading={(taskState[rec.name] || {}).loading || false}
                done={(taskState[rec.name] || {}).done || false}
                progressMsg={(taskState[rec.name] || {}).progress}
              />
              {(taskState[rec.name] || {}).error && (
                <div className="mt-1 ml-10 px-3 py-2 rounded-md border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/20">
                  <p className="text-xs text-red-600 dark:text-red-400 leading-relaxed break-words">
                    收集角色失败：{(taskState[rec.name] || {}).error}
                  </p>
                </div>
              )}
            </div>
          );
        })}
        </div>
      )}
    </div>
  );
}
