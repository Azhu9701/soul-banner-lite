import { createFileRoute, Link, notFound } from '@tanstack/react-router'
import { useEffect, useMemo, useRef, useState } from "react"
import {
  fetchSessionDetail,
  fetchSessionDigest,
  fetchSessionAnnotations,
  triggerDistill,
  extractRecommendedSouls,
  deleteMessagesFromSeq,
  type Annotation,
  type SessionDetail,
  type SessionDigest,
} from "@/lib/api"
import { SESSIONS_UPDATED_EVENT } from "@/components/sidebar-sessions"
import {
  ArrowLeft,
  User,
  ChevronDown,
  ChevronUp,
  Trash2,
  RefreshCw,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import SessionActions from "@/components/session-actions"
import FollowUpInput from "@/components/follow-up-input"
import { SoulRecommendationCard } from "@/components/soul-recommendation-card"
import { SoulResponseCard } from "@/components/soul-response-card"
import { SynthesisSection } from "@/components/synthesis-section"
import { BreadcrumbSetter } from "@/components/breadcrumb-setter"
import { MessageForkButton } from "@/components/message-fork-button"

import { MODE_LABELS_LONG, MODE_COLORS_TEXT, type PossessionMode } from "@/config/possession-modes"
import { Skeleton } from "@/components/ui/skeleton"

export const Route = createFileRoute('/sessions/$id')({
  component: SessionDetailPage,
})

function SessionDetailPage() {
  const { id } = Route.useParams()
  const search = Route.useSearch() as { fork?: string }
  const isFork = search?.fork === "true"
  const mountedRef = useRef(true)
  useEffect(() => { return () => { mountedRef.current = false } }, [])

  const [digest, setDigest] = useState<SessionDigest | null>(null)
  const [digestError, setDigestError] = useState(false)
  const [annotations, setAnnotations] = useState<Annotation[]>([])
  const [detail, setDetail] = useState<SessionDetail | null>(null)
  const [expanded, setExpanded] = useState<boolean | null>(null)
  const [scrollToSeq, setScrollToSeq] = useState<number | null>(null)
  const [distilling, setDistilling] = useState(false)
  const [followUpTrigger, setFollowUpTrigger] = useState<{ question: string; soul?: string } | null>(null)

  const sessionSoulNames = useMemo(() => {
    if (!detail) return []
    const names = new Set<string>()
    for (const m of detail.messages) {
      if (m.soul_name && m.role !== "system") names.add(m.soul_name)
    }
    return [...names]
  }, [detail])

  const autoDistilledRef = useRef(false)

  useEffect(() => {
    fetchSessionDigest(id).then((d) => { if (mountedRef.current) setDigest(d) }).catch(() => { if (mountedRef.current) setDigestError(true) })
    fetchSessionDetail(id, true).then((d) => { if (mountedRef.current) setDetail(d) }).catch(() => {})
    fetchSessionAnnotations(id).then((a) => { if (mountedRef.current) setAnnotations(a) }).catch(() => {})
  }, [id])

  useEffect(() => {
    if (!digest || autoDistilledRef.current) return
    if ((digest.status === 'completed' || digest.status === 'active') && digest.digest_at === null) {
      autoDistilledRef.current = true
      setDistilling(true)
      triggerDistill(id).finally(() => {
        setTimeout(() => {
          if (!mountedRef.current) return
          fetchSessionDigest(id).then((d) => { if (mountedRef.current) setDigest(d) }).catch(() => {})
          if (mountedRef.current) setDistilling(false)
        }, 3000)
      })
    }
  }, [digest, id])

  useEffect(() => {
    const handle = () => {
      if (!mountedRef.current) return
      fetchSessionDigest(id).then((d) => { if (mountedRef.current) setDigest(d) }).catch(() => {})
      fetchSessionAnnotations(id).then((a) => { if (mountedRef.current) setAnnotations(a) }).catch(() => {})
    }
    window.addEventListener(SESSIONS_UPDATED_EVENT, handle)
    return () => window.removeEventListener(SESSIONS_UPDATED_EVENT, handle)
  }, [id])

  useEffect(() => {
    if (digest && expanded === null) {
      setExpanded(digest.observations.length === 0)
    }
  }, [digest, expanded])

  useEffect(() => {
    if (detail && scrollToSeq !== null) {
      requestAnimationFrame(() => {
        const el = document.getElementById(`msg-${scrollToSeq}`)
        if (el) {
          el.scrollIntoView({ behavior: "smooth", block: "center" })
          el.classList.add("ring-2", "ring-orange-400/60", "transition-shadow")
          setTimeout(() => el.classList.remove("ring-2", "ring-orange-400/60"), 1800)
        }
        setScrollToSeq(null)
      })
    }
  }, [detail, scrollToSeq])

  const handleJumpToMessage = (seq: number | null) => {
    if (seq === null) return
    if (!expanded) setExpanded(true)
    setScrollToSeq(seq)
  }

  const handleDistill = async () => {
    setDistilling(true)
    try {
      await triggerDistill(id)
      setTimeout(() => {
        if (!mountedRef.current) return
        fetchSessionDigest(id).then((d) => { if (mountedRef.current) setDigest(d) }).catch(() => {})
        if (mountedRef.current) setDistilling(false)
      }, 3000)
    } catch {
      if (mountedRef.current) setDistilling(false)
    }
  }

  if (digestError) throw notFound()
  if (!digest) return <Skeleton className="h-96" />

  const hasObservations = digest.observations.length > 0

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <BreadcrumbSetter label={digest.title} />

      <div className="flex items-center gap-3">
        <Link to="/sessions">
          <Button variant="ghost" size="icon" className="h-8 w-8">
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div className="flex-1 min-w-0">
          <h1 className="text-xl font-bold truncate">{digest.title}</h1>
          <p className="text-sm text-muted-foreground">
            {(MODE_LABELS_LONG as Record<string, string>)[digest.mode] || digest.mode}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8"
          onClick={handleDistill}
          disabled={distilling}
          title={hasObservations ? "重新压缩为 observation" : "压缩为 observation"}
        >
          <RefreshCw className={`h-4 w-4 ${distilling ? "animate-spin" : ""}`} />
        </Button>
        <SessionActions sessionId={id} title={digest.title} />
      </div>

      {hasObservations && (
        <div className="rounded-xl border bg-gradient-to-br from-orange-50/40 to-amber-50/20 dark:from-orange-950/15 dark:to-amber-950/5 p-5 space-y-4">
          <div className="flex items-center justify-between flex-wrap gap-2">
            <h3 className="text-sm font-semibold flex items-center gap-2">
              本次会话精华 · {digest.observations.length} 条 observation
            </h3>
            <span className="text-xs text-muted-foreground font-mono">
              省 {(digest.savings_ratio * 100).toFixed(0)}% tokens ·{" "}
              {digest.total_read_tokens.toLocaleString()}/{digest.total_work_tokens.toLocaleString()}
            </span>
          </div>

          {digest.summary && (
            <div className="text-sm text-muted-foreground italic border-l-2 border-orange-300/60 dark:border-orange-700/60 pl-3">
              "{digest.summary}"
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {digest.observations.map((o) => {
              const jumpable = o.source_seq !== null
              return (
                <button
                  key={o.id}
                  type="button"
                  onClick={() => handleJumpToMessage(o.source_seq)}
                  disabled={!jumpable}
                  className={`text-left rounded-lg border bg-background/80 p-3 space-y-1.5 transition-all ${
                    jumpable
                      ? "hover:shadow-sm hover:bg-background hover:border-orange-300/60 cursor-pointer"
                      : "cursor-default opacity-90"
                  }`}
                  title={jumpable ? `跳转到原始消息 #${o.source_seq}` : undefined}
                >
                  <div className="flex items-start gap-2">
                    <div className="flex-1 min-w-0">
                      <h4 className="text-sm font-semibold leading-tight">{o.title}</h4>
                      {o.soul_name && (
                        <span className="text-[10px] text-muted-foreground">— {o.soul_name}</span>
                      )}
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed pl-7 whitespace-pre-wrap">
                    {o.content}
                  </p>
                </button>
              )
            })}
          </div>
        </div>
      )}

      {annotations.length > 0 && (
        <div className="rounded-xl border bg-gradient-to-br from-purple-50/40 to-indigo-50/20 dark:from-purple-950/15 dark:to-indigo-950/5 p-5 space-y-4">
          <h3 className="text-sm font-semibold flex items-center gap-2">
            角色间互批 · {annotations.length} 条 marginalia
          </h3>
          <div className="space-y-3">
            {annotations.map((a) => (
              <div key={a.id} className="rounded-lg border bg-background/80 p-3 space-y-2">
                <div className="flex items-center gap-2 text-xs">
                  <span className="font-semibold">{a.source_soul}</span>
                  <span className="text-muted-foreground">→</span>
                  <span className="font-semibold">{a.target_soul}</span>
                  <span className="ml-auto text-[10px] uppercase tracking-wider opacity-50 font-mono">
                    {a.kind}
                  </span>
                </div>
                <blockquote className="text-xs text-muted-foreground italic border-l-2 border-purple-300/40 pl-2 line-clamp-3">
                  "{a.target_excerpt}"
                </blockquote>
                <p className="text-sm leading-relaxed whitespace-pre-wrap">{a.comment}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="flex justify-center">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setExpanded(!expanded)}
          className="text-muted-foreground hover:text-foreground"
        >
          {expanded ? (
            <ChevronUp className="h-4 w-4 mr-1" />
          ) : (
            <ChevronDown className="h-4 w-4 mr-1" />
          )}
          {expanded ? "收起完整对话" : "展开完整对话"}
        </Button>
      </div>

      {expanded && (detail ? <FullConversation detail={detail} sessionId={id} onReload={() => fetchSessionDetail(id, true).then(setDetail)} onSummonSoul={(name, subtask) => setFollowUpTrigger({ question: subtask || "", soul: name })} onRefresh={(question) => setFollowUpTrigger({ question })} /> : <Skeleton className="h-96" />)}

      {isFork && (
        <div className="rounded-xl border border-orange-200 dark:border-orange-800 bg-orange-50 dark:bg-orange-950/20 p-4 space-y-1">
          <p className="text-sm font-medium text-orange-700 dark:text-orange-300">
            已从上一条消息分叉，保留了 {detail?.messages.length ?? 0} 条历史记录
          </p>
          <p className="text-xs text-muted-foreground">
            在下方输入你的新问题，将以上下文为起点继续讨论
          </p>
        </div>
      )}
      <FollowUpInput sessionId={id} trigger={followUpTrigger} sessionSouls={sessionSoulNames} />
    </div>
  );
}

function FullConversation({
  detail,
  sessionId,
  onReload,
  onSummonSoul,
  onRefresh,
}: {
  detail: SessionDetail;
  sessionId: string;
  onReload?: () => void;
  onSummonSoul?: (name: string, subtask?: string) => void;
  onRefresh?: (question: string) => void;
}) {
  const { messages } = detail;
  const [deleting, setDeleting] = useState<number | null>(null);
  const [refreshing, setRefreshing] = useState<number | null>(null);
  const [followUpLimit, setFollowUpLimit] = useState(5);

  const handleDelete = async (seq: number) => {
    setDeleting(seq);
    await deleteMessagesFromSeq(sessionId, seq + 1);
    setDeleting(null);
    onReload?.();
  };

  const handleRefresh = async (seq: number, content: string) => {
    setRefreshing(seq);
    await deleteMessagesFromSeq(sessionId, seq + 1);
    setRefreshing(null);
    onRefresh?.(content);
  };

  const {
    sorted, userMsgs, soulMsgs, synthMsgs, sysMsgs,
    soulResponses, initUserMsgs, initSynths, followPairs,
  } = useMemo(() => {
    const sorted = [...messages].sort(
      (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
    );
    const userMsgs = sorted.filter((m) => m.role === "user");
    const soulMsgs = sorted.filter(
      (m) => (m.role === "assistant" || m.role === "soul") && m.soul_name && m.soul_name !== "知识卡片"
    );
    const synthMsgs = sorted.filter((m) => m.role === "synthesis");
    const sysMsgs = sorted.filter((m) => m.role === "system" && !m.content.startsWith("[REVIEW]") && !m.content.startsWith("## ⏳"));

    const soulResponses: Record<string, string> = {};
    for (const m of soulMsgs) {
      const name = m.soul_name!;
      soulResponses[name] = (soulResponses[name] ? soulResponses[name] + "\n\n" : "") + m.content;
    }
    const firstSynth = synthMsgs[0];
    const initUserMsgs = firstSynth
      ? userMsgs.filter((m) => new Date(m.created_at).getTime() < new Date(firstSynth.created_at).getTime())
      : userMsgs;
    const followUserMsgs = firstSynth
      ? userMsgs.filter((m) => new Date(m.created_at).getTime() > new Date(firstSynth.created_at).getTime())
      : [];
    const followPairs: { question: typeof userMsgs[number]; answer: typeof synthMsgs[number] | null }[] = [];
    for (const q of followUserMsgs) {
      const qTime = new Date(q.created_at).getTime();
      const answer = synthMsgs.find((s) => new Date(s.created_at).getTime() > qTime);
      followPairs.push({ question: q, answer: answer || null });
    }
    const followSynthIds = new Set(followPairs.filter((p) => p.answer).map((p) => p.answer!.id));
    const initSynths = synthMsgs.filter((s) => !followSynthIds.has(s.id));

    return { sorted, userMsgs, soulMsgs, synthMsgs, sysMsgs, soulResponses, initUserMsgs, initSynths, followPairs };
  }, [messages]);

  const recommendedSouls = useMemo(() => {
    if (initSynths.length === 0) return [];
    return extractRecommendedSouls(initSynths[0].content);
  }, [initSynths]);

  const sessionSoulNames = useMemo(() => {
    const names = new Set<string>();
    for (const m of soulMsgs) {
      if (m.soul_name) names.add(m.soul_name);
    }
    return [...names];
  }, [soulMsgs]);

  return (
    <div className="space-y-6 border-t pt-6">
      {initUserMsgs.map((msg) => (
        <div key={msg.id} id={`msg-${msg.seq}`} className="group flex gap-3 flex-row-reverse scroll-mt-20 rounded-xl">
          <div className="shrink-0">
            <div className="w-9 h-9 rounded-full bg-primary/10 flex items-center justify-center">
              <User className="h-4 w-4 text-primary" />
            </div>
          </div>
          <div className="max-w-[70%]">
            <div className="flex items-center gap-2 mb-1 flex-row-reverse">
              <span className="text-xs font-semibold text-muted-foreground">用户</span>
              <span className="text-xs text-muted-foreground">
                {new Date(msg.created_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
              </span>
            </div>
            <div className="relative">
              <div className="rounded-xl p-4 bg-primary/5 border border-primary/10">
                <p className="text-sm leading-relaxed whitespace-pre-wrap">{msg.content}</p>
              </div>
              <div className="absolute -left-2 top-1/2 -translate-y-1/2 translate-x-[-100%] opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-0.5 -space-x-px">
                <MessageForkButton sessionId={detail.session.id} messageSeq={msg.seq} />
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-7 w-7 text-muted-foreground hover:text-blue-500"
                  onClick={() => handleRefresh(msg.seq, msg.content)}
                  disabled={refreshing === msg.seq}
                  title="重新生成回复"
                >
                  <RefreshCw className="h-3.5 w-3.5" />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-7 w-7 text-muted-foreground hover:text-destructive"
                  onClick={() => handleDelete(msg.seq)}
                  disabled={deleting === msg.seq}
                  title="删除此条及之后的所有消息"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          </div>
        </div>
      ))}

      {Object.keys(soulResponses).length > 0 && (
        <div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {Object.entries(soulResponses).map(([name, content]) => (
              <SoulResponseCard key={name} name={name} content={content} />
            ))}
          </div>
        </div>
      )}

      {sysMsgs.map((msg) => (
        <div key={msg.id} id={`msg-${msg.seq}`} className="text-center py-2 scroll-mt-20 rounded-md">
          <span className="text-xs text-muted-foreground">{msg.content}</span>
        </div>
      ))}

      {initSynths.map((msg) => (
        <div key={msg.id} id={`msg-${msg.seq}`} className="scroll-mt-20 rounded-xl">
          <SynthesisSection
            messages={[{ id: msg.id, content: msg.content, created_at: msg.created_at }]}
          />
        </div>
      ))}

      {recommendedSouls.length > 0 && (
        <SoulRecommendationCard recommendations={recommendedSouls} onSummonSoul={onSummonSoul} sessionSouls={sessionSoulNames} />
      )}

      {followPairs.length > 0 && (
        <div className="space-y-6 border-t pt-6">
          <h3 className="text-sm font-semibold text-muted-foreground">追问记录</h3>
          {followPairs.slice(0, followUpLimit).map(({ question, answer }) => (
            <div key={question.id} className="space-y-4">
              <div id={`msg-${question.seq}`} className="group flex gap-3 flex-row-reverse scroll-mt-20 rounded-xl">
                <div className="shrink-0">
                  <div className="w-9 h-9 rounded-full bg-primary/10 flex items-center justify-center">
                    <User className="h-4 w-4 text-primary" />
                  </div>
                </div>
                <div className="max-w-[70%]">
                  <div className="flex items-center gap-2 mb-1 flex-row-reverse">
                    <span className="text-xs font-semibold text-muted-foreground">追问</span>
                    <span className="text-xs text-muted-foreground">
                      {new Date(question.created_at).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
                    </span>
                  </div>
                  <div className="relative">
                    <div className="rounded-xl p-4 bg-primary/5 border border-primary/10">
                      <p className="text-sm leading-relaxed whitespace-pre-wrap">{question.content}</p>
                    </div>
                    <div className="absolute -left-2 top-1/2 -translate-y-1/2 translate-x-[-100%] opacity-0 group-hover:opacity-100 transition-opacity">
                      <div className="flex items-center gap-0.5">
                        <MessageForkButton sessionId={detail.session.id} messageSeq={question.seq} />
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-7 w-7 text-muted-foreground hover:text-blue-500"
                          onClick={() => handleRefresh(question.seq, question.content)}
                          disabled={refreshing === question.seq}
                          title="重新生成追问回复"
                        >
                          <RefreshCw className="h-3.5 w-3.5" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-7 w-7 text-muted-foreground hover:text-destructive"
                          onClick={() => handleDelete(question.seq)}
                          disabled={deleting === question.seq}
                          title="删除此条及之后的所有消息"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
              {answer && (
                <div id={`msg-${answer.seq}`} className="scroll-mt-20 rounded-xl">
                  <SynthesisSection
                    messages={[{ id: answer.id, content: answer.content, created_at: answer.created_at }]}
                  />
                </div>
              )}
            </div>
          ))}
          {followPairs.length > followUpLimit && (
            <div className="text-center py-3">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setFollowUpLimit(followUpLimit + 10)}
              >
                加载更多追问（剩余 {followPairs.length - followUpLimit} 条）
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
