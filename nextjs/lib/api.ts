export const API_BASE = process.env.NEXT_PUBLIC_API_URL || "/api/v1";

const API_TOKEN = process.env.NEXT_PUBLIC_API_TOKEN || "";

interface ApiError extends Error {
  status?: number;
  url?: string;
  operation?: string;
}

interface ApiRequestOptions extends RequestInit {
  operation?: string;
  /** 超时时间 (ms)，默认 30s */
  timeout?: number;
  /** 重试次数，默认 3 次（仅网络错误时重试） */
  retries?: number;
}

class NetworkError extends Error {
  constructor(message: string, public cause?: unknown) {
    super(message);
    this.name = "NetworkError";
  }
}

async function apiRequest<T>(
  endpoint: string,
  options: ApiRequestOptions = {}
): Promise<T> {
  const { operation, timeout = 30000, retries = 3, ...fetchOptions } = options;
  const url = `${API_BASE}${endpoint}`;
  const opName = operation || endpoint;

  const headers = new Headers(fetchOptions.headers);
  if (API_TOKEN && !headers.has("Authorization")) {
    headers.set("Authorization", `Bearer ${API_TOKEN}`);
  }

  let lastError: unknown;

  for (let attempt = 0; attempt <= retries; attempt++) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    // 合并外部 signal 和超时 signal
    const existingSignal = fetchOptions.signal;
    let onExternalAbort: (() => void) | undefined;
    if (existingSignal) {
      onExternalAbort = () => controller.abort();
      existingSignal.addEventListener("abort", onExternalAbort);
    }

    try {
      const res = await fetch(url, { ...fetchOptions, headers, signal: controller.signal });

      if (!res.ok) {
        let errorDetail = res.statusText;
        try {
          const errorData = await res.json();
          errorDetail = errorData.message || errorData.error || errorDetail;
        } catch { }

        const error = new Error(`API request failed: ${opName} - ${errorDetail}`) as ApiError;
        error.status = res.status;
        error.url = url;
        error.operation = opName;
        throw error;
      }

      if (res.status === 204) {
        return undefined as T;
      }

      return res.json();
    } catch (err: unknown) {
      lastError = err;
      clearTimeout(timer);

      // 非网络错误不重试
      if (err instanceof Error && err.name !== "TypeError") {
        // HTTP 错误（有 status code）
        if ((err as ApiError).status) throw err;
        // 超时 / AbortError
        if (err.name === "AbortError") throw new Error(`请求超时: ${opName} — 服务器 ${timeout / 1000}s 内未响应`);
      }

      // 如果是外部 signal 导致的 abort，不重试
      if (existingSignal?.aborted) {
        throw err;
      }

      // 最后一次尝试，抛出错误
      if (attempt === retries) {
        throw new NetworkError(
          `网络请求失败: ${opName} (已重试 ${retries} 次) — ${err instanceof Error ? err.message : String(err)}`,
          err
        );
      }

      // 指数退避: 1s, 2s, 4s（热点场景需要更长时间恢复）
      const delay = Math.min(1000 * Math.pow(2, attempt), 8000);
      console.warn(`[apiRequest] ${opName} 网络错误，${delay}ms 后重试 (${attempt + 1}/${retries})`, err);
      await new Promise((r) => setTimeout(r, delay));
    } finally {
      if (existingSignal && onExternalAbort) {
        existingSignal.removeEventListener("abort", onExternalAbort);
      }
    }
  }

  throw lastError;
}

export interface SoulListEntry {
  name: string;
  ismism_code: string;
  field: string;
  domains: string[];
  tags: string[];
  summon_count: number;
  trigger_keywords: string[];
  self_declare: string;
  model: string;
  compat: string[];
  incompat: string[];
}

export interface SoulProfile {
  name: string;
  ismism_code: string;
  field: string;
  ontology: string;
  epistemology: string;
  teleology: string;
  domains: string[];
  tags: string[];
  exclude_scenarios: string[];
  summon_prompt: string;
  summon_count: number;
  effectiveness: { effective: number; partial: number; invalid: number };
  created_at: string;
  updated_at: string;
  practice_observations: PracticeObservation[];
  title: string;
  description: string;
  voice: string;
  mind: string;
  self_declare: string;
  skills_expertise: string[];
  model: string;
  tools: string;
  trigger_keywords: string[];
  compat: string[];
  incompat: string[];
}

export interface PracticeObservation {
  date: string;
  observation: string;
  revision_type: "Confirmed" | "Modified" | "Overturned";
}

export type ConferenceEvent =
  | { type: "soul_token";   soul: string; token: string }
  | { type: "soul_done";    soul: string }
  | { type: "soul_error";   soul: string; error: string }
  | { type: "synthesis_chunk"; content: string }
  | { type: "synthesis_done" }
  | { type: "collision";    from: string; to: string; content: string }
  | { type: "synthesis_started" }
  | { type: "cost";         llm_calls: number; tokens_used: number; estimated_cost: string }
  | { type: "done" }
  | { type: "session_started"; mode: string }
  | { type: "soul_started"; soul: string }
  | { type: "system";       message: string }
  | { type: "error";        message: string; soul?: string }

export interface KnowledgeResult {
  soul_name: string | null;
  content_snippet: string;
  mode: string;
  task_summary: string;
  created_at: string;
  session_id: string;
}

export interface KnowledgeTopic {
  session_id: string;
  title: string;
  mode: string;
  created_at: string;
  soul_names: string[];
  card_summary: string | null;
  synthesis_preview: string | null;
}

export interface KnowledgeCardItem {
  id: string;
  title: string;
  content: string;
  source_soul: string | null;
  source_session: string | null;
  tags: string[];
  created_at: string;
  updated_at: string;
}

export async function fetchSouls(): Promise<SoulListEntry[]> {
  return apiRequest<SoulListEntry[]>('/souls', {
    next: { revalidate: 60 },
    operation: 'fetchSouls',
  });
}

export async function fetchSoul(name: string): Promise<SoulProfile> {
  return apiRequest<SoulProfile>(
    `/souls/${encodeURIComponent(name)}`,
    { next: { revalidate: 60 }, operation: 'fetchSoul' }
  );
}

export async function deleteSoul(name: string): Promise<void> {
  return apiRequest<void>(
    `/souls/${encodeURIComponent(name)}`,
    { method: 'DELETE', operation: 'deleteSoul' }
  );
}

export async function updateSoul(
  name: string,
  data: Record<string, unknown>
): Promise<void> {
  return apiRequest<void>(
    `/souls/${encodeURIComponent(name)}`,
    {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
      operation: 'updateSoul',
    }
  );
}

export interface AutoCreateAccepted {
  task_id: string;
  soul_name: string;
}

export interface AutoCreateWsEvent {
  task_id: string;
  soul_name: string;
  phase: 'collecting' | 'refining' | 'done' | 'error';
  message?: string;
  profile?: SoulProfile;
}

/** POST /souls/auto-create — returns immediately with task_id. Progress via WS. */
export async function autoCreateSoul(name: string): Promise<AutoCreateAccepted> {
  return apiRequest<AutoCreateAccepted>('/souls/auto-create', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
    operation: 'autoCreateSoul',
    // fast response — WS handles progress
  });
}

/**
 * Connect to auto-create progress WebSocket.
 * Resolves when `done` or rejects on `error`.
 */
export function watchAutoCreate(
  taskId: string,
  soulName: string,
  onProgress: (evt: AutoCreateWsEvent) => void
): { abort: () => void } {
  const wsHost = API_BASE.replace('http://', 'ws://').replace('/api/v1', '');
  const ws = new WebSocket(`${wsHost}/ws/souls/auto-create/${taskId}`);
  let settled = false;

  ws.onmessage = (e) => {
    try {
      const evt = JSON.parse(e.data) as AutoCreateWsEvent;
      onProgress(evt);
      if (evt.phase === 'done' || evt.phase === 'error') {
        settled = true;
        ws.close();
      }
    } catch {}
  };

  ws.onerror = () => {
    if (!settled) {
      settled = true;
      onProgress({
        task_id: taskId,
        soul_name: soulName,
        phase: 'error',
        message: 'WebSocket 连接失败',
      });
    }
  };

  ws.onclose = () => {
    if (!settled) {
      settled = true;
      onProgress({
        task_id: taskId,
        soul_name: soulName,
        phase: 'error',
        message: '连接意外关闭',
      });
    }
  };

  return { abort: () => ws.close() };
}

// ── Analytics ──

export interface SummonStatsResponse {
  total_calls: number;
  unique_souls_called: number;
  total_souls_available: number;
  total_tokens: number;
  by_mode: Record<string, number>;
  by_soul: SoulCallStats[];
}

export interface SoulCallStats {
  soul_name: string;
  call_count: number;
  effective_count: number;
  partial_count: number;
  invalid_count: number;
  total_tokens: number;
}

export interface SoulAlert {
  soul_name: string;
  alert_type: string;
  detail: string;
}

export interface BoundaryReview {
  soul_name: string;
  effective_rate: number;
  total_calls: number;
  threshold: number;
  recommendation: string;
}


export async function fetchSummonStats(): Promise<SummonStatsResponse> {
  return apiRequest<SummonStatsResponse>('/analytics/summon-stats', {
    next: { revalidate: 60 },
    operation: 'fetchSummonStats',
  });
}

export async function fetchModeDistribution(): Promise<Record<string, number>> {
  return apiRequest<Record<string, number>>('/analytics/mode-distribution', {
    next: { revalidate: 60 },
    operation: 'fetchModeDistribution',
  });
}

export async function fetchUnsummonedAlerts(days = 30): Promise<SoulAlert[]> {
  return apiRequest<SoulAlert[]>(
    `/analytics/unsummoned?threshold_days=${days}`,
    { next: { revalidate: 60 }, operation: 'fetchUnsummonedAlerts' }
  );
}

export async function fetchLowEffectiveness(threshold = 0.3): Promise<BoundaryReview[]> {
  return apiRequest<BoundaryReview[]>(
    `/analytics/low-effectiveness?threshold=${threshold}`,
    { next: { revalidate: 60 }, operation: 'fetchLowEffectiveness' }
  );
}


// ── Sessions ──

export interface SessionSummary {
  id: string;
  title: string;
  mode: string;
  status: string;
  created_at: string;
  message_count: number;
  soul_count: number;
  total_tokens: number;
  digest_summary: string | null;
  observation_count: number;
}

export interface SessionDetail {
  session: {
    id: string;
    title: string;
    mode: string;
    status: string;
    created_at: string;
    updated_at: string;
  };
  messages: Message[];
}

export interface Message {
  id: string;
  session_id: string;
  role: string;
  soul_name: string | null;
  content: string;
  seq: number;
  created_at: string;
}

export async function fetchSessions(
  limit = 50,
  offset = 0,
  noCache = false,
): Promise<SessionSummary[]> {
  return apiRequest<SessionSummary[]>(
    `/sessions?limit=${limit}&offset=${offset}`,
    noCache
      ? { cache: 'no-store', operation: 'fetchSessions' }
      : { next: { revalidate: 60 }, operation: 'fetchSessions' }
  );
}

export async function fetchSessionDetail(
  id: string,
  noCache = false,
): Promise<SessionDetail> {
  return apiRequest<SessionDetail>(
    `/sessions/${id}`,
    noCache
      ? { cache: 'no-store', operation: 'fetchSessionDetail' }
      : { next: { revalidate: 60 }, operation: 'fetchSessionDetail' }
  );
}

export async function deleteSession(id: string): Promise<void> {
  return apiRequest<void>(`/sessions/${id}`, {
    method: 'DELETE',
    operation: 'deleteSession',
  });
}

export interface BatchDeleteResult {
  deleted: number;
  errors: string[];
}

export async function batchDeleteSessions(ids: string[]): Promise<BatchDeleteResult> {
  return apiRequest<BatchDeleteResult>('/sessions/batch-delete', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ids }),
    operation: 'batchDeleteSessions',
  });
}

export async function renameSession(id: string, title: string): Promise<void> {
  return apiRequest<void>(`/sessions/${id}/rename`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title }),
    operation: 'renameSession',
  });
}

export async function deleteMessagesFromSeq(sessionId: string, seq: number): Promise<{ deleted: number }> {
  return apiRequest<{ deleted: number }>(`/sessions/${sessionId}/messages/${seq}`, {
    method: 'DELETE',
    operation: 'deleteMessagesFromSeq',
  });
}

// ── Fork ──

export interface ForkResponse {
  session_id: string;
  forked_message_count: number;
}

export async function forkSession(
  sessionId: string,
  fromMessageSeq: number,
  task?: string,
): Promise<ForkResponse> {
  return apiRequest<ForkResponse>(`/sessions/${sessionId}/fork`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ from_message_seq: fromMessageSeq, task }),
    operation: 'forkSession',
  });
}

// ── Possess ──

export interface AnalyzeResponse {
  entry_type: string;
  matched_souls: { name: string; field: string; ismism_code: string; rationale: string }[];
  recommended_mode: string;
  task_cards?: Record<string, string>;
}

export interface AnalyzeStreamEvent {
  phase: "classifying" | "matching" | "matched" | "review_done" | "adjusting" | "practice_opening" | "analysis_content" | "done";
  entry_type?: string;
  souls?: { name: string; field: string; ismism_code: string; rationale: string }[];
  mode?: string;
  task_cards?: Record<string, string>;
  source?: string;
  is_done?: boolean;
  content?: string;
  response?: AnalyzeResponse;
}

export async function analyzeTask(
    task: string,
    signal?: AbortSignal,
    onEvent?: (event: AnalyzeStreamEvent) => void,
): Promise<AnalyzeResponse> {
    const url = `${API_BASE}/possess/analyze`;
    const res = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ task }),
        signal,
    });

    if (!res.ok) {
        let errorDetail = res.statusText;
        try { const ed = await res.json(); errorDetail = ed.message || ed.error || errorDetail; } catch {}
        throw new Error(`API request failed: analyzeTask - ${errorDetail}`);
    }

    if (!res.body) throw new Error("analyzeTask: empty response body");
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";
    let finalResponse: AnalyzeResponse | null = null;
    let eventCount = 0;

    // 保存中间状态以便在未收到 done 事件时构造响应
    let intermediateEntryType = "conventional";
    let intermediateSouls: AnalyzeResponse["matched_souls"] = [];
    let intermediateMode = "single";

    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() || "";

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed.startsWith("data:")) continue;
            try {
                const event: AnalyzeStreamEvent = JSON.parse(trimmed.slice(5).trim());
                eventCount++;
                // 保存中间状态
                if (event.entry_type) intermediateEntryType = event.entry_type;
                if (event.souls) intermediateSouls = event.souls;
                if (event.mode) intermediateMode = event.mode;
                if (event.phase === "done" && event.response) {
                    finalResponse = event.response;
                }
                onEvent?.(event);
            } catch {}
        }
    }

    // 如果没有收到 done 事件，尝试 fallback 机制
    if (!finalResponse) {
        // Fallback 1: 从中间状态构造响应
        if (intermediateSouls.length > 0) {
            console.warn("analyzeTask: stream ended without done event, constructing response from intermediate state");
            finalResponse = {
                entry_type: intermediateEntryType,
                matched_souls: intermediateSouls,
                recommended_mode: intermediateMode,
                task_cards: {}
            };
        }
        // Fallback 2: 旧版后端返回的是纯 JSON 而非 SSE
        else if (eventCount === 0 && buffer.trim()) {
            try {
                const legacyResponse = JSON.parse(buffer.trim()) as AnalyzeResponse;
                finalResponse = legacyResponse;
            } catch {}
        }
    }

    if (!finalResponse) {
        throw new Error("analyzeTask: stream ended without done event");
    }
    return finalResponse;
}

export interface StartPossessionResponse {
  session_id: string;
  ws_url: string;
}

export async function startPossession(params: {
  task: string;
  mode?: string;
  souls: string[];
  task_cards?: Record<string, string>;
  search_topic?: boolean;
  judgment?: string;
  worry?: string;
  unknown?: string;
  interrogation_context?: string;
}): Promise<StartPossessionResponse> {
  return apiRequest<StartPossessionResponse>('/possess', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
    operation: 'startPossession',
  });
}

/** POST /possess/court — 一键启动模拟仲裁庭（5角色 conference） */
export async function startCourtSession(params: {
  task: string;
  judgment?: string;
  worry?: string;
  unknown?: string;
}): Promise<StartPossessionResponse> {
  return apiRequest<StartPossessionResponse>('/possess/court', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
    operation: 'startCourtSession',
  });
}

export async function startBusinessSession(params: {
  task: string;
  judgment?: string;
  worry?: string;
  unknown?: string;
}): Promise<StartPossessionResponse> {
  return apiRequest<StartPossessionResponse>('/possess/business', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
    operation: 'startBusinessSession',
  });
}

export async function exportSessionMarkdown(id: string, title: string): Promise<void> {
  const url = `${API_BASE}/sessions/${id}/export/markdown`;
  // window.open 触发浏览器原生下载行为，允许用户选择保存位置
  window.open(url, '_blank');
}

// ── Knowledge ──

export async function searchKnowledge(query: string, limit = 20): Promise<KnowledgeResult[]> {
  return apiRequest<KnowledgeResult[]>(
    `/knowledge/search?q=${encodeURIComponent(query)}&limit=${limit}`,
    { operation: 'searchKnowledge' }
  );
}

export async function rebuildFts(): Promise<{ indexed: number }> {
  return apiRequest<{ indexed: number }>('/knowledge/rebuild', {
    method: 'POST',
    operation: 'rebuildFts',
  });
}

export async function fetchKnowledgeTopics(params?: {
  mode?: string;
  limit?: number;
  offset?: number;
}): Promise<KnowledgeTopic[]> {
  const searchParams = new URLSearchParams();
  if (params?.mode) searchParams.set('mode', params.mode);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  const qs = searchParams.toString();
  return apiRequest<KnowledgeTopic[]>(
    `/knowledge/topics${qs ? `?${qs}` : ''}`,
    { operation: 'fetchKnowledgeTopics' }
  );
}

export async function fetchKnowledgeCards(params?: {
  soul?: string;
  limit?: number;
  offset?: number;
}): Promise<KnowledgeCardItem[]> {
  const searchParams = new URLSearchParams();
  if (params?.soul) searchParams.set('soul', params.soul);
  if (params?.limit) searchParams.set('limit', String(params.limit));
  if (params?.offset) searchParams.set('offset', String(params.offset));
  const qs = searchParams.toString();
  return apiRequest<KnowledgeCardItem[]>(
    `/knowledge/cards${qs ? `?${qs}` : ''}`,
    { operation: 'fetchKnowledgeCards' }
  );
}

// ── Synthesis (structured output §9.5) ──

export interface SynthesisOutput {
  consensus: ConsensusItem[];
  divergence: DivergenceItem[];
  blind_spots: BlindSpotItem[];
  principal_contradiction: Contradiction;
  action_program: ActionItem[];
}

export interface ConsensusItem {
  point: string;
  shared_by: string[];
}

export interface DivergenceItem {
  axis: string;
  positions: Position[];
}

export interface Position {
  soul_name: string;
  stance: string;
}

export interface BlindSpotItem {
  dimension: string;
  missing_perspective: string;
  coverable_by_existing: boolean;
  suggested_soul: string | null;
}

export interface Contradiction {
  description: string;
  parties: string[];
}

export interface ActionItem {
  direction: string;
  rationale: string;
  priority: number;
}

// ── Domain Profile (领域模式切换) ──

export interface DomainOption {
  profile: string;
  label: string;
  available: boolean;
}

export interface DomainInfo {
  profile: string;
  system_name: string;
  agent_noun: string;
  synthesis_verb: string;
  dimensions: string[];
  available: DomainOption[];
  enabled_modes: string[];
}

export async function getDomainInfo(): Promise<DomainInfo> {
  return apiRequest<DomainInfo>('/config/domain', {
    cache: 'no-store',
    operation: 'getDomainInfo',
  });
}

export async function setDomain(profile: string): Promise<DomainInfo> {
  return apiRequest<DomainInfo>('/config/domain', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ profile }),
    operation: 'setDomain',
  });
}

// ── DeepSeek Cache Hint ──

export interface CacheHint {
  provider: "deepseek";
  cache_optimized: boolean;
  estimated_discount: string; // "80-92%"
}

export interface OcrResult {
  filename: string;
  text: string | null;
  error: string | null;
}

export async function ocrFiles(files: File[]): Promise<OcrResult[]> {
  const form = new FormData();
  files.forEach((f) => form.append("files", f));
  const res = await fetch(`${API_BASE}/possess/ocr`, {
    method: "POST",
    body: form,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || `OCR failed: ${res.statusText}`);
  }
  const data = await res.json();
  return data.results;
}

// ── SearXNG ──

export interface SearxngResultItem {
  title: string;
  url: string;
  content: string;
  engine: string;
  engines: string[];
  score: number;
  category: string;
}

export interface SearxngSearchResponse {
  query: string;
  number_of_results: number;
  results: SearxngResultItem[];
  suggestions: string[];
  unresponsive_engines: string[][];
}

export async function searchWeb(params: {
  q: string;
  pageno?: number;
  language?: string;
  categories?: string;
}): Promise<SearxngSearchResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('q', params.q);
  if (params.pageno) searchParams.set('pageno', String(params.pageno));
  if (params.language) searchParams.set('language', params.language);
  if (params.categories) searchParams.set('categories', params.categories);
  return apiRequest<SearxngSearchResponse>(
    `/searxng/search?${searchParams.toString()}`,
    { operation: 'searchWeb' }
  );
}

// ── Session Observations (claude-mem digest) ──

export interface SessionObservation {
  id: string;
  session_id: string;
  soul_name: string | null;
  obs_type: string;
  title: string;
  content: string;
  source_seq: number | null;
  read_tokens: number;
  work_tokens: number;
  confidence: number;
  created_at: string;
}

export interface SessionDigest {
  session_id: string;
  title: string;
  mode: string;
  status: string;
  created_at: string;
  summary: string | null;
  digest_at: string | null;
  observations: SessionObservation[];
  total_read_tokens: number;
  total_work_tokens: number;
  savings_ratio: number;
}

const OBS_TYPE_EMOJI: Record<string, string> = {
  session: '\u{1F3AF}',
  discovery: '\u{1F535}',
  decision: '⚖️',
  bugfix: '\u{1F534}',
  feature: '\u{1F7E3}',
  refactor: '\u{1F504}',
  change: '✅',
  security: '\u{1F6A8}',
};

export function obsEmoji(type: string): string {
  return OBS_TYPE_EMOJI[type] ?? '\u{1F4CB}';
}





export async function fetchSessionDigest(id: string): Promise<SessionDigest> {
  return apiRequest<SessionDigest>(`/sessions/${id}/digest`, {
    operation: 'fetchSessionDigest',
  });
}

export async function triggerDistill(id: string): Promise<{ ok: boolean; message: string }> {
  return apiRequest<{ ok: boolean; message: string }>(`/sessions/${id}/distill`, {
    method: 'POST',
    operation: 'triggerDistill',
  });
}

// ── Marginalia annotations (post-conference annotation pass) ──

export interface Annotation {
  id: string;
  session_id: string;
  /** 写批注的魂 */
  source_soul: string;
  /** 被批注的魂 */
  target_soul: string;
  /** 引用的原文片段 */
  target_excerpt: string;
  /** 批注内容 */
  comment: string;
  /** disagree | extend | nuance | question | support | ... */
  kind: string;
  created_at: string;
}

export async function fetchSessionAnnotations(id: string): Promise<Annotation[]> {
  return apiRequest<Annotation[]>(`/sessions/${id}/annotations`, {
    cache: 'no-store',
    operation: 'fetchSessionAnnotations',
  });
}

// ── Interrogation gate (审查官入场反问) ──

export interface InterrogationQuestion {
  text: string;
  required: boolean;
}

export interface InterrogationResponse {
  gate_id: string;
  questions: InterrogationQuestion[];
  message?: string;
}

export interface InterrogationVerdictResponse {
  passed: boolean;
  reason: string;
  questions?: InterrogationQuestion[];  // 驳回时的追加反问
  refined_task?: string;               // 通过时的改写 task
}

export async function startInterrogation(task: string): Promise<InterrogationResponse> {
  return apiRequest<InterrogationResponse>('/possess/interrogate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ task }),
    operation: 'startInterrogation',
    timeout: 120000,
  });
}

export async function submitInterrogation(
  gateId: string,
  answers: { question_index: number; answer: string }[],
): Promise<InterrogationVerdictResponse> {
  return apiRequest<InterrogationVerdictResponse>(
    `/possess/interrogate/${gateId}/respond`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ answers }),
      operation: 'submitInterrogation',
      timeout: 120000,
    },
  );
}

// ── Recommended souls parser (mirrors rust/possession/src/modes/conference.rs:extract_recommended_souls) ──

export interface ParsedSoulRecommendation {
  name: string;
  rationale: string;
  subtask?: string;
}

// Keywords that mean "structural metadata", never a soul name
const RESERVED_LABELS = new Set([
  "推荐理由", "理由", "推荐子任务", "子任务",
  "需要补充", "无需补充", "推荐补充", "补充方向",
]);

/**
 * Parse "## 七、推荐补充魂" section from synthesis content.
 *
 * Handles two layouts:
 * (A) Inline:   `1. **庄子** — 推荐理由：xxx 推荐子任务：**yyy**`
 * (B) Nested:   `1. **庄子**` followed by `   - **推荐理由**：xxx` / `   - **推荐子任务**：yyy`
 *
 * Algorithm:
 * - Split the section by **top-level** numbered/dashed items (col 0 indent)
 * - Within each item body, scan for `**推荐理由**：xxx` and `**推荐子任务**：xxx`
 * - Reject items whose extracted name is in RESERVED_LABELS (catches nested **推荐理由** etc)
 */
export function extractRecommendedSouls(synthesis: string): ParsedSoulRecommendation[] {
  // 保护：超过 50KB 的综合文本不进行正则解析（防止对巨型内容的灾难性回溯导致 OOM）
  if (synthesis.length > 50_000) return [];

  const sectionStart = synthesis.indexOf("## 七、推荐补充魂");
  if (sectionStart === -1) return [];

  const fromStart = synthesis.slice(sectionStart);
  const nextHeading = fromStart.slice(1).indexOf("\n## ");
  const sectionText = nextHeading !== -1 ? fromStart.slice(0, nextHeading + 1) : fromStart;

  if (sectionText.includes("无需补充")) return [];

  // Top-level item header at column 0: `- **name**`, `* **name**`, `1. **name**` etc.
  // We derive item bodies in a second pass to avoid JS regex's lack of \Z anchor.
  const TOP_ITEM_HEADER_RE = /^(?:[-*]|\d+\.)\s+\*\*([^*\n]{1,80}?)\*\*[ \t]*(.*)$/gm;

  // Field extractors inside an item body
  // Lookahead terminates at: nested sub-bullet `- **...**`, next top-level item `1. **` or `- **` at column 0,
  // another `**推荐...**` label, or end of body
  const FIELD_END = "(?=\\n[ \\t]*[-*]\\s+\\*\\*|\\n(?:[-*]|\\d+\\.)\\s+\\*\\*|\\n\\s*\\*\\*推荐子任务\\*\\*|\\n\\s*\\*\\*推荐理由\\*\\*|\\n\\s*\\*\\*[^*\\n]{1,40}\\*\\*\\s*[：:]?|\\n\\s*推荐子任务\\s*[：:]|\\n\\s*---|$)";
  const RATIONALE_BOLD_RE = new RegExp("\\*\\*(?:推荐理由|理由)\\*\\*\\s*[：:]\\s*([\\s\\S]+?)" + FIELD_END);
  const SUBTASK_BOLD_RE = new RegExp("\\*\\*(?:推荐子任务|子任务)\\*\\*\\s*[：:]\\s*([\\s\\S]+?)" + FIELD_END);
  // Inline (no leading `- **label**`)
  const SUBTASK_INLINE_BOLD_RE = /推荐子任务\s*[：:]\s*\*\*([^*\n]+?)\*\*/;
  const SUBTASK_INLINE_PLAIN_RE = /推荐子任务\s*[：:]\s*([^\n。！？]+?)(?=[。！？\n]|$)/;
  const RATIONALE_INLINE_RE = /推荐理由\s*[：:]\s*([\s\S]+?)(?=推荐子任务\s*[：:]|$)/;

  const stripWrap = (s: string) => s.trim().replace(/^[「""'']|[」""'']$/g, "").trim();
  const cleanTail = (s: string) => s.replace(/\s*[。.，,;；:：]+\s*$/, "").trim();

  // Pass 1: collect header positions
  type HeaderPos = { name: string; tailStart: number; lineEnd: number };
  const headers: HeaderPos[] = [];
  let hm: RegExpExecArray | null;
  TOP_ITEM_HEADER_RE.lastIndex = 0;
  while ((hm = TOP_ITEM_HEADER_RE.exec(sectionText)) !== null) {
    const name = hm[1].trim();
    if (!name || name.length >= 80) continue;
    if (RESERVED_LABELS.has(name)) continue;
    headers.push({
      name,
      tailStart: hm.index + hm[0].length - hm[2].length,
      lineEnd: hm.index + hm[0].length,
    });
  }

  const out: ParsedSoulRecommendation[] = [];
  const seen = new Set<string>();
  for (let i = 0; i < headers.length; i++) {
    const h = headers[i];
    if (seen.has(h.name)) continue;
    seen.add(h.name);

    const nextStart = i + 1 < headers.length
      ? sectionText.lastIndexOf("\n", headers[i + 1].tailStart) + 1
      : sectionText.length;
    const bodyEnd = nextStart > h.tailStart ? nextStart : sectionText.length;
    const body = sectionText.slice(h.tailStart, bodyEnd);

    let rationale = "";
    let subtask: string | undefined;

    // Try nested layout first (sub-bullet **推荐子任务**: ...)
    const subBold = body.match(SUBTASK_BOLD_RE);
    if (subBold) {
      subtask = stripWrap(subBold[1]).replace(/^\*\*|\*\*$/g, "").trim();
    } else {
      const subInlineBold = body.match(SUBTASK_INLINE_BOLD_RE);
      const subInlinePlain = body.match(SUBTASK_INLINE_PLAIN_RE);
      if (subInlineBold) {
        subtask = stripWrap(subInlineBold[1]);
      } else if (subInlinePlain) {
        subtask = stripWrap(subInlinePlain[1]);
      }
    }

    // Rationale: try sub-bullet first, then inline, then whole body
    const ratBold = body.match(RATIONALE_BOLD_RE);
    if (ratBold) {
      rationale = ratBold[1].trim();
    } else {
      const ratInline = body.match(RATIONALE_INLINE_RE);
      rationale = ratInline ? ratInline[1].trim() : body.trim();
    }

    rationale = rationale.replace(/推荐子任务\s*[：:][\s\S]*$/, "").trim();
    rationale = rationale.replace(/^\s*[—\-:：]+\s*/, "");
    rationale = cleanTail(rationale);

    // Last item may include section-level closing paragraphs. Strip them.
    if (i === headers.length - 1) {
      // Non-indented paragraphs after the structured fields = section conclusion
      rationale = rationale.replace(/\n\n(?![ \t]+(?:[-*]|\d+\.)\s)[\s\S]*$/, "").trim();
    }

    out.push({
      name: h.name,
      rationale: rationale || "综合官推荐补充",
      ...(subtask ? { subtask } : {}),
    });
  }
  return out;
}
