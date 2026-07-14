"use client";

import { useState, useCallback, useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  searchKnowledge,
  rebuildFts,
  fetchKnowledgeTopics,
  fetchKnowledgeCards,
  type KnowledgeResult,
  type KnowledgeTopic,
  type KnowledgeCardItem,
} from "@/lib/api";
import { Search, RefreshCw, Loader2, Tag, User, Calendar } from "lucide-react";
import { ModeBarChart } from "@/components/mode-bar-chart";

type TabKey = "cards" | "topics" | "search";

const MODE_LABELS: Record<string, string> = {
  single: "单角色",
  conference: "合议",
  debate: "辩论",
  relay: "接力",
  learn: "学习",
  practice_opening: "实践开口",
};

export function KnowledgeBrowser() {
  const navigate = useNavigate();
  const [tab, setTab] = useState<TabKey>("topics");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildMsg, setRebuildMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [topics, setTopics] = useState<KnowledgeTopic[]>([]);
  const [topicsLoaded, setTopicsLoaded] = useState(false);
  const [cards, setCards] = useState<KnowledgeCardItem[]>([]);
  const [cardsLoaded, setCardsLoaded] = useState(false);
  const [searchResults, setSearchResults] = useState<KnowledgeResult[]>([]);
  const [searched, setSearched] = useState(false);

  const [modeFilter, setModeFilter] = useState<string>("");

  useEffect(() => {
    loadTopics();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadTopics = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchKnowledgeTopics({
        mode: modeFilter || undefined,
        limit: 50,
      });
      setTopics(data);
      setTopicsLoaded(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    } finally {
      setLoading(false);
    }
  }, [modeFilter]);

  const loadCards = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await fetchKnowledgeCards({ limit: 50 });
      setCards(data);
      setCardsLoaded(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    } finally {
      setLoading(false);
    }
  }, []);

  const doSearch = useCallback(async (q: string) => {
    if (!q.trim()) {
      setSearchResults([]);
      setSearched(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const data = await searchKnowledge(q, 30);
      setSearchResults(data);
      setSearched(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "搜索失败");
    } finally {
      setLoading(false);
    }
  }, []);

  const handleTabChange = (key: TabKey) => {
    setTab(key);
    setRebuildMsg(null);
    setError(null);
    if (key === "topics" && !topicsLoaded) loadTopics();
    if (key === "cards" && !cardsLoaded) loadCards();
  };

  const handleRebuild = async () => {
    setRebuilding(true);
    setRebuildMsg(null);
    try {
      const res = await rebuildFts();
      setRebuildMsg(`索引重建完成，共 ${res.indexed} 条消息`);
      if (tab === "topics") loadTopics();
      else if (tab === "cards") loadCards();
      else if (query.trim()) doSearch(query);
    } catch (e) {
      setError(e instanceof Error ? e.message : "重建失败");
    } finally {
      setRebuilding(false);
    }
  };

  const handleSearchKeyDown = (e: React.KeyboardEvent) => {
    if (e.nativeEvent.isComposing || e.keyCode === 229) return;
    if (e.key === "Enter") {
      if (query.trim()) {
        setTab("search");
        doSearch(query);
      } else {
        handleTabChange("topics");
      }
    }
  };

  const tabDefs: { key: TabKey; label: string }[] = [
    { key: "topics", label: "分析报告" },
    { key: "cards", label: "知识卡片" },
  ];

  const modes = Array.from(
    new Set(topics.map((t) => t.mode).filter(Boolean))
  ).sort();

  const modeCounts: Record<string, number> = {};
  if (tab === "topics") {
    topics.forEach((t) => {
      modeCounts[t.mode] = (modeCounts[t.mode] || 0) + 1;
    });
  } else if (tab === "search" && searchResults.length > 0) {
    searchResults.forEach((r) => {
      modeCounts[r.mode] = (modeCounts[r.mode] || 0) + 1;
    });
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex gap-2">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleSearchKeyDown}
            placeholder="搜索角色输出、综合报告、会话记录..."
            className="w-full rounded-lg border bg-background pl-10 pr-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary/30"
          />
        </div>
        <button
          onClick={handleRebuild}
          disabled={rebuilding}
          className="flex items-center gap-2 rounded-lg border px-4 py-2.5 text-sm hover:bg-muted transition-colors disabled:opacity-50"
        >
          {rebuilding ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="h-4 w-4" />
          )}
          重建索引
        </button>
      </div>

      {error && (
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {rebuildMsg && (
        <div className="rounded-lg border border-emerald-500/50 bg-emerald-500/10 p-3 text-sm text-emerald-700 dark:text-emerald-300">
          {rebuildMsg}
        </div>
      )}

      <div className="flex items-center justify-between border-b">
        <div className="flex gap-1">
          {tabDefs.map((t) => (
            <button
              key={t.key}
              onClick={() => handleTabChange(t.key)}
              className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                tab === t.key
                  ? "border-primary text-primary"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        {tab === "topics" && (
          <div className="flex items-center gap-2 pb-1">
            <select
              value={modeFilter}
              onChange={(e) => setModeFilter(e.target.value)}
              className="text-xs rounded-lg border bg-background px-2 py-1 focus:outline-none focus:ring-2 focus:ring-primary/30"
            >
              <option value="">全部模式</option>
              {Object.entries(MODE_LABELS).map(([k, v]) => (
                <option key={k} value={k}>
                  {v}
                </option>
              ))}
            </select>
            <button
              onClick={loadTopics}
              className="text-xs text-primary hover:underline"
            >
              筛选
            </button>
          </div>
        )}
      </div>

      {loading && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground py-4">
          <Loader2 className="h-4 w-4 animate-spin" />
          加载中...
        </div>
      )}

      {!loading && Object.keys(modeCounts).length > 0 && (
        <ModeBarChart data={modeCounts} />
      )}

      {tab === "topics" && !loading && topicsLoaded && (
        <>
          {topics.length === 0 ? (
            <div className="text-sm text-muted-foreground py-8 text-center">
              暂无分析报告，请先进行合议或辩论对话
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="text-xs text-muted-foreground">
                {topics.length} 个会话
              </div>
              {topics.map((t) => (
                <div
                  key={t.session_id}
                  role="button"
                  tabIndex={0}
                  onClick={() => navigate({ to: `/sessions/${t.session_id}` })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter")
                      navigate({ to: `/sessions/${t.session_id}` });
                  }}
                  className="rounded-lg border bg-background p-4 transition-colors hover:border-primary/20 cursor-pointer"
                >
                  <h3 className="text-sm font-semibold mb-1.5 line-clamp-1">
                    {t.title}
                  </h3>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground mb-2">
                    <span className="rounded-full bg-muted px-2 py-0.5">
                      {MODE_LABELS[t.mode] || t.mode}
                    </span>
                    <Calendar className="h-3 w-3" />
                    <span>{t.created_at?.slice(0, 10)}</span>
                  </div>
                  {t.soul_names.length > 0 && (
                    <div className="flex items-center gap-1 mb-2">
                      <User className="h-3 w-3 text-muted-foreground" />
                      <span className="text-xs text-muted-foreground">
                        {t.soul_names.slice(0, 8).join("、")}
                        {t.soul_names.length > 8 ? "…" : ""}
                      </span>
                    </div>
                  )}
                  <p className="text-xs text-muted-foreground line-clamp-3">
                    {t.synthesis_preview || t.card_summary || "暂无预览"}
                  </p>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {tab === "cards" && !loading && cardsLoaded && (
        <>
          {cards.length === 0 ? (
            <div className="text-sm text-muted-foreground py-8 text-center">
              暂无知识卡片，请先进行合议对话生成卡片
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="text-xs text-muted-foreground">
                {cards.length} 张卡片
              </div>
              {cards.map((c) => (
                <div
                  key={c.id}
                  role="button"
                  tabIndex={0}
                    onClick={() =>
                      c.source_session &&
                      navigate({ to: `/sessions/${c.source_session}` })
                    }
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && c.source_session)
                      navigate({ to: `/sessions/${c.source_session}` });
                  }}
                  className="rounded-lg border bg-background p-4 transition-colors hover:border-primary/20 cursor-pointer"
                >
                  <h3 className="text-sm font-semibold mb-1.5">{c.title}</h3>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground mb-2">
                    <Calendar className="h-3 w-3" />
                    <span>{c.created_at?.slice(0, 10)}</span>
                    {c.tags.length > 0 && (
                      <>
                        <Tag className="h-3 w-3 ml-2" />
                        <span>{c.tags.slice(0, 5).join("、")}</span>
                      </>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground line-clamp-3">
                    {c.content.length > 300
                      ? c.content.slice(0, 300) + "..."
                      : c.content}
                  </p>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {tab === "search" && searched && !loading && (
        <>
          {searchResults.length === 0 ? (
            <div className="text-sm text-muted-foreground py-8 text-center">
              未找到匹配结果
            </div>
          ) : (
            <div className="flex flex-col gap-3">
              <div className="text-xs text-muted-foreground">
                {searchResults.length} 条结果
              </div>
              {searchResults.map((r, i) => (
                <div
                  key={i}
                  role="button"
                  tabIndex={0}
                  onClick={() => navigate({ to: `/sessions/${r.session_id}` })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter")
                      navigate({ to: `/sessions/${r.session_id}` });
                  }}
                  className="rounded-lg border bg-background p-4 transition-colors hover:border-primary/20 cursor-pointer"
                >
                  <div className="flex items-center gap-2 mb-2">
                    {r.soul_name && (
                      <span className="text-sm font-semibold">
                        {r.soul_name}
                      </span>
                    )}
                    <span className="text-xs rounded-full bg-muted px-2 py-0.5 text-muted-foreground">
                      {r.mode}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {r.created_at?.slice(0, 10)}
                    </span>
                  </div>
                  {r.task_summary && (
                    <p className="text-xs text-muted-foreground mb-1">
                      任务: {r.task_summary}
                    </p>
                  )}
                  <p
                    className="text-sm leading-relaxed"
                  >
                    {r.content_snippet}
                  </p>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {!loading && tab !== "search" && !topicsLoaded && !cardsLoaded && (
        <div className="text-sm text-muted-foreground py-8 text-center">
          请选择一个选项卡浏览知识库
        </div>
      )}
    </div>
  );
}
