# 向量 Hybrid 搜索设计 (v2.1)

- **日期**:2026-06-18
- **状态**:已确认,待实现
- **范围**:`foundation` 新增向量召回 + 跨表 RRF 融合;`archive`/`api` 转发层适配;HTTP `/knowledge/search` 扩展为跨表搜索
- **vs v1 的变更**:① embedding 后端从进程内 ONNX 改为 **LMStudio HTTP(OpenAI 兼容)**;② 索引范围从 messages 扩展到 **messages + KnowledgeCard**;③ `/knowledge/search` 改为**跨表 RRF**
- **vs v2 的变更(v2.1)**:embedding 模型从 Qwen3-Embedding-4B 换为 **BGE-M3**(568M,维度 1024,8192 上下文,三模式能力)。理由:进程内/HTTP 部署双灵活、8192 token 对长 message 友好、多语言、未来可探索用其 sparse 模式补充 FTS5。架构主体不变,只改配置与维度

---

## 1. 背景与目标

### 1.1 现状

rust-agent 的 knowledge 搜索当前依赖 **SQLite FTS5 + trigram tokenizer**(`rust/foundation/src/sqlite.rs:622-683`),配合 `cjk_tokenize`(`sqlite.rs:586-599`)处理中文。

但"知识库"在项目里**名不副实地分散在三套独立系统**:

| 表面 | 数据源 | 当前搜索 | 实质 |
|------|--------|---------|------|
| `search_knowledge` | `messages.content` | FTS5 ✅ | **名字叫 knowledge 但搜的是聊天消息** |
| `KnowledgeCard` | `knowledge_cards` 表 | **无搜索** ❌ 仅 filter 列表 | conference 合议后 LLM 蒸馏的 ≤500 字精华卡片 |
| `KnowledgeTopic` | sessions+messages 派生视图 | **无搜索** ❌ 仅 filter 列表 | 非实表,实时 N+1 计算 |

`rust/foundation/src/vector_search.rs` 存在 `SimpleVectorIndex` 内存脚手架(线性扫描余弦),**无 embedding 模型接入,生产无调用**。

### 1.2 问题

1. **FTS5 是字面匹配,无法语义召回**——同义改述、跨语言、口语化表达全部漏召
2. **KnowledgeCard 完全无法搜索**——conference 蒸馏的高价值卡片只能按 `source_soul` filter 翻页,用户找不到
3. 三个 surface 共用 "knowledge" 前缀却互相独立,用户体验割裂

### 1.3 目标

在现有 FTS5 基础上**并联向量召回**,覆盖 messages + KnowledgeCard 两个 surface,用 **RRF 跨表融合**;`/knowledge/search` 一个端点搜全部,返回带 `source` 字段区分来源。

embedding 走 **LMStudio OpenAI 兼容接口**(项目已集成),跑 **BGE-M3**(568M,1024 维,8192 token 上下文,三模式能力)。全链路失败时**静默降级纯 FTS5**,老用户行为零回归。

### 1.4 非目标

- 不替换 FTS5(它是底线)
- **不索引 KnowledgeTopic**——它是从 messages 派生的视图,embedding 与 message 重叠,边际收益低;先做 message+card
- 不引外部向量数据库(qdrant/lancedb/pgvector)
- 不引新 embedding 二进制依赖(`ort`/`tokenizers`)——走 HTTP 复用现有 LMStudio 基建

---

## 2. 关键决策(已与用户确认,v2 修订点加 ⚠️)

| # | 决策 | 选择 | 理由 |
|---|------|------|------|
| 1 | ⚠️ **embedding 后端** | **LMStudio HTTP(OpenAI 兼容 `/v1/embeddings`)** | 项目已集成 LMStudio(`rust/ai-gateway/src/lmstudio.rs`),走 `http://localhost:1234`,换模型只改配置;不引 `ort`/`tokenizers` 重依赖 |
| 2 | ⚠️ **embedding 模型** | **BGE-M3**(568M,维度 1024,8192 token 上下文) | vs Qwen3-4B:进程内/HTTP 部署双灵活、8192 token 对长 message 友好、100+ 多语言、原生 dense+sparse+multi-vector 三模式(未来可探索替代 FTS5);中文质量长期霸榜,Qwen3-4B 仅在 benchmark 略胜但用户感知不到 |
| 3 | 业务线 | **knowledge 优先** | souls 数量小现有够用;knowledge 数据持续增长,痛点最大 |
| 4 | ⚠️ **索引范围** | **messages + KnowledgeCard** | v1 只覆盖 messages;调查发现 KnowledgeCard 完全无搜索,高价值却不可发现,纳入刻不容缓 |
| 5 | 向量存储 | **SQLite-vec(`vec0` 虚表,同库)** | 跟 FTS5 同库同事务同备份;消息/卡片各一张 vec0 表(主键不冲突) |
| 6 | 融合策略 | **RRF(k=60),跨表融合** | BM25 负 rank 与 cosine 量纲不可比,RRF 用 rank 不用 score,20 行 Rust |
| 7 | ⚠️ **查询入口** | **扩展 `/knowledge/search` 跨表** | 一个端点搜 messages + cards,RRF 融合两源,返回带 `source` 字段;符合用户"一个搜索框"直觉 |
| 8 | 默认开关 | **`vector_search.enabled: false`** | 零回归保证;LMStudio 在跑 + 手动开启才生效 |

---

## 3. 架构总览

```
                    ┌──────────────────────────────────────────────────┐
   GET /api/knowledge/search?q=...                  Hybrid Search (跨表)
   POST /api/knowledge/rebuild        ┌──────────────────────────────────┐
                │                     │  ArchiveSystem                   │
                ▼                     │  .search_knowledge_hybrid()      │
   ┌────────────────────┐              │     │                             │
   │ routes/knowledge.rs│──── call ────┼─────┤                             │
   └────────────────────┘              │     ▼                             │
                                       │  HybridSearcher                  │
                                       │     │                             │
                                       │     ├─[FTS5 路径]                  │
                                       │     │   search_messages_fts()      │
                                       │     │   search_cards_fts() ← 新表  │
                                       │     │                              │
                                       │     ├─[向量路径]                   │
                                       │     │   EmbeddingClient (HTTP)     │
                                       │     │     ↓ BGE-M3 via LMStudio     │
                                       │     │   search_message_vectors()   │
                                       │     │   search_card_vectors()      │
                                       │     │                              │
                                       │     └─► RRF Fuse (k=60) 跨 4 路    │
                                       │           │                        │
                                       │           ▼                        │
                                       │     Vec<KnowledgeResult + source> │
                                       └──────────────────────────────────┘
```

### 3.1 分层归属(严格遵循现有分层)

- **`foundation`**(底层 + 推理 + 融合)
  - `SqliteDb` 扩展:两张 vec0 表 + 各自 KNN;新增 `cards_fts` 表(给 cards 补 FTS5)
  - 新增 `EmbeddingClient`:HTTP 客户端,调 OpenAI 兼容 `/v1/embeddings`
  - 新增 `HybridSearcher`:跨表 RRF 融合 + 降级控制
- **`archive`**:`ArchiveSystem::search_knowledge_hybrid` 调 HybridSearcher
- **`api`**:`routes/knowledge.rs` 签名小改(返回结构加 `source` 字段)

### 3.2 核心约束

- 向量相关失败一律降级,**FTS5 是底线**
- 两张 vec0 表主键不冲突:`message_embeddings(session_id, seq)` + `card_embeddings(card_id)`
- 卡片写入同步 embed(conference 流程已经在 await,加一次 HTTP 调用即可,不像 message 那样需要异步 spawn)
- 跨表 RRF 统一用 `(surface, key)` 二元组做去重 key

---

## 4. 组件细节

### 4.1 `EmbeddingClient`(`foundation/src/embedding.rs`,新建)

**职责**:HTTP 客户端,调 OpenAI 兼容 `/v1/embeddings`,把文本变成 1024 维向量(BGE-M3 默认输出)。**不加载任何模型文件**。

```rust
pub struct EmbeddingClient {
    http: reqwest::Client,
    base_url: String,      // 默认 http://localhost:1234 (LMStudio)
    api_key: Option<String>,
    model: String,         // 默认 "bge-m3"(LMStudio pull 的名字)
    dim: usize,            // 1024,启动时探测一次
}

impl EmbeddingClient {
    pub fn new(base_url: &str, api_key: Option<&str>, model: &str) -> Self;
    pub async fn probe_dim(&self) -> Result<usize>;   // 启动时 embed("ping") 探测维度
    pub async fn embed(&self, text: &str) -> Result<Embedding>;
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>>;
    pub fn dim(&self) -> usize;
}
```

**请求体**(标准 OpenAI 格式):
```json
POST {base_url}/v1/embeddings
{ "model": "bge-m3", "input": ["text1", "text2"] }
→ { "data": [{ "embedding": [...] }, ...] }
```

**关键决策**:

- 复用 `reqwest`(项目已用),不引新 HTTP 依赖
- **维度不硬编码**——`probe_dim` 在启动时 embed 一句 "ping" 探测实际维度(BGE-M3 默认 1024,但留口子兼容未来换模型)。vec0 表创建用探测值
- **BGE-M3 长文本处理**——8192 token 上下文足够覆盖绝大多数 message;极少数超长 message(>8192)由 LMStudio 内部截断,我们不预处理
- 失败返回 `Err`,由上层降级;client 自己不重试(embedding 不是关键路径,失败就降级)
- 超时 10s(LMStudio BGE-M3 GPU 推理通常 <100ms,CPU ~200ms,留足余量)

### 4.2 `SqliteDb` 向量扩展(改 `foundation/src/sqlite.rs`)

**职责**:两张 vec0 表 + 两张 FTS5 表 + 各自 KNN/MATCH。

#### 4.2.1 新增表

```sql
-- 启动时 load sqlite-vec 扩展
-- 消息向量(主键 session_id + seq)
CREATE VIRTUAL TABLE IF NOT EXISTS message_embeddings USING vec0(
    session_id TEXT, seq INTEGER, embedding float[{DIM}]
);

-- 卡片向量(主键 card_id)
CREATE VIRTUAL TABLE IF NOT EXISTS card_embeddings USING vec0(
    card_id TEXT, embedding float[{DIM}]
);

-- 卡片全文索引(给 KnowledgeCard 补 FTS5,现在没有)
CREATE VIRTUAL TABLE IF NOT EXISTS cards_fts USING fts5(
    card_id, title, content, tokenize='trigram'
);
```

`{DIM}` 用 `probe_dim` 探测值替换。

#### 4.2.2 新方法

```rust
impl SqliteDb {
    fn ensure_vec0(conn: &Connection, dim: usize) -> Result<()>;
    fn ensure_cards_fts(conn: &Connection) -> Result<()>;   // 类比 ensure_fts5

    // ── 写入 ──
    pub fn index_message_embedding(&self, session_id: &str, seq: i64, emb: &[f32]) -> Result<()>;
    pub fn index_card_embedding(&self, card_id: &str, emb: &[f32]) -> Result<()>;
    pub fn index_card_fts(&self, card_id: &str, title: &str, content: &str) -> Result<()>;

    // ── 向量 KNN ──
    pub fn search_message_vectors(&self, q: &[f32], limit: usize) -> Result<Vec<VectorHit>>;
    pub fn search_card_vectors(&self, q: &[f32], limit: usize) -> Result<Vec<VectorHit>>;

    // ── FTS5(卡片是新的)──
    pub fn search_cards_fts(&self, query: &str, limit: usize) -> Result<Vec<CardFtsHit>>;
    // 现有 search_knowledge 重命名为 search_messages_fts(内部方法,保持行为)

    // ── 重建 ──
    pub fn rebuild_message_vectors(&self, embs: &[(String, i64, Vec<f32>)]) -> Result<usize>;
    pub fn rebuild_card_vectors(&self, embs: &[(String, Vec<f32>)]) -> Result<usize>;
    pub fn rebuild_cards_fts(&self) -> Result<usize>;

    // ── 回表 ──
    pub fn fetch_messages_by_keys(&self, keys: &[(String, i64)]) -> Result<Vec<KnowledgeResult>>;
    pub fn fetch_cards_by_ids(&self, ids: &[String]) -> Result<Vec<KnowledgeResult>>;
}

pub struct VectorHit {
    pub key: String,       // message: "session_id|seq";card: "card_id"
    pub distance: f32,
}
```

**关键决策**:

- `sqlite-vec` 扩展二进制(`.dylib`/`.so`/`.dll`)随项目分发到 `rust/foundation/extensions/`
- KNN 走 `WHERE embedding MATCH ? ORDER BY distance LIMIT ?`
- **`insert_knowledge_card` 加 FTS hook**——conference 创建卡片后,同步写 `cards_fts`(纯 DB 操作,在 Storage 层内完成)
- **卡片 embedding 不在 `insert_knowledge_card` 内**——DB 层不依赖 HTTP;embedding 由服务层 `ArchiveSystem::index_knowledge_card` 异步 spawn(见 §5.2)

### 4.3 `HybridSearcher`(`foundation/src/hybrid_search.rs`,新建)

**职责**:跨表 RRF 融合 4 路召回(messages FTS + messages vec + cards FTS + cards vec),统一返回带 `source` 字段。

```rust
pub struct HybridSearcher {
    db: Arc<SqliteDb>,
    embedding: Arc<EmbeddingClient>,
    enabled: Arc<AtomicBool>,   // 运行期开关,失败降级时翻 false
}

#[derive(Serialize)]
pub struct HybridResult {
    #[serde(flatten)]
    pub inner: KnowledgeResult,
    pub source: ResultSource,   // Message | Card
}

impl HybridSearcher {
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<HybridResult>> {
        let k = std::cmp::min(limit * 3, 200);

        // 1. embed query(失败则向量两路全空,RRF 退化纯 FTS5)
        let query_vec = if self.enabled.load() {
            self.embedding.embed(query).await.ok()
        } else { None };

        // 2. 并行召回 4 路(tokio::join!)
        let (msg_fts, msg_vec, card_fts, card_vec) = tokio::try_join!(
            self.db.search_messages_fts(query, k),
            self.db.search_message_vectors_opt(query_vec.as_ref(), k),
            self.db.search_cards_fts(query, k),
            self.db.search_card_vectors_opt(query_vec.as_ref(), k),
        )?;

        // 3. 跨表 RRF(key 用 (surface, id) 二元组,天然不冲突)
        let fused = rrf_fuse_cross_table(
            vec![("message", msg_fts), ("card", card_fts)],
            vec![("message", msg_vec), ("card", card_vec)],
            limit,
        );

        // 4. 分组回表(批量,避免 N+1)
        let msg_keys: Vec<_> = fused.iter().filter(|(s,_,_)| *s=="message")
            .map(|(_,k,_)| parse_msg_key(k)).collect();
        let card_ids: Vec<_> = fused.iter().filter(|(s,_,_)| *s=="card")
            .map(|(_,k,_)| k.clone()).collect();
        let (msgs, cards) = tokio::try_join!(
            self.db.fetch_messages_by_keys(&msg_keys),
            self.db.fetch_cards_by_ids(&card_ids),
        )?;

        // 5. 按 RRF 顺序拼装 + 标 source
        Ok(assemble_in_order(fused, msgs, cards))
    }
}
```

**跨表 RRF 实现**(~30 行,纯函数):

```rust
fn rrf_fuse_cross_table(
    fts_routes: Vec<(&str, Vec<VectorHit>)>,
    vec_routes: Vec<(&str, Vec<VectorHit>)>,
    limit: usize,
) -> Vec<(String, String, f32)> {   // (surface, key, score)
    let mut scores: HashMap<(String, String), f32> = HashMap::new();
    for (surface, hits) in &fts_routes {
        for (rank, h) in hits.iter().enumerate() {
            *scores.entry((surface.to_string(), h.key.clone())).or_default()
                += 1.0 / (60 + rank as f32 + 1.0);
        }
    }
    for (surface, hits) in &vec_routes {
        for (rank, h) in hits.iter().enumerate() {
            *scores.entry((surface.to_string(), h.key.clone())).or_default()
                += 1.0 / (60 + rank as f32 + 1.0);
        }
    }
    let mut all: Vec<_> = scores.into_iter()
        .map(|((s,k), v)| (s, k, v))
        .collect();
    all.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    all.into_iter().take(limit).collect()
}
```

**降级控制全在这一层**:

- `embedding.embed(query)` 失败 → `warn!` + 向量两路返回空,RRF 退化纯 FTS5(数学等价)
- 任一 `search_*` DB 失败 → 该路返回空,其余继续
- 4 路全失败 → 返回 `Err` → HTTP 500
- `enabled=false`(运行期) → 跳过 embed,只跑 FTS5 两路

### 4.4 `Storage` trait 扩展(`foundation/src/storage.rs`)

```rust
// 新增:返回带 source 的跨表结果
async fn search_knowledge_hybrid(&self, query: &str, limit: usize)
    -> Result<Vec<crate::hybrid_search::HybridResult>>;

// 新增:卡片写入后触发 FTS+embedding(由 conference 调)
async fn index_knowledge_card(&self, card_id: &str, title: &str, content: &str) -> Result<()>;

// 旧 search_knowledge 保留(行为不变,内部委托 hybrid 但只取 message source)
async fn search_knowledge(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeResult>>;
```

### 4.5 返回结构扩展

现有 `KnowledgeResult` 加可选 `source` 字段(向后兼容,serde 默认 Message):

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeResult {
    pub soul_name: Option<String>,
    pub content_snippet: String,
    pub mode: String,
    pub task_summary: String,
    pub created_at: String,
    pub session_id: String,
    #[serde(default = "default_source")]
    pub source: String,   // "message" | "card",默认 "message"
    pub card_id: Option<String>,   // 仅 card source 有值
    pub tags: Option<Vec<String>>, // 仅 card source 有值
}
```

---

## 5. 数据流

### 5.1 写入路径 — Messages(实时,异步 spawn)

```
possession 写一条 message
        │
        ▼
SqliteDb::append_message(msg)
        │
        ├─[同步] INSERT messages + INSERT knowledge_fts   ← 现有,不动
        │
        └─[异步 spawn] 向量写入(新增)
                │
                ▼
        EmbeddingClient::embed(msg.content)  HTTP→LMStudio
                │
                ▼
        SqliteDb::index_message_embedding(session_id, seq, vec)
                │
                ▼
        [失败?] warn! + 丢弃,不影响主流程
```

**关键决策**:
- 异步 spawn(消息是关键路径,embedding 失败不阻塞对话)
- 幂等:写入前 `DELETE WHERE session_id=? AND seq=?`
- 空/极短消息跳过
- spawn 拿 `Arc<EmbeddingClient>` + `Arc<SqliteDb>`

### 5.2 写入路径 — KnowledgeCards(同步,conference 流程内)

```
conference.rs:card_fut 完成 → store.insert_knowledge_card()
        │
        ▼
[新增] ArchiveSystem::index_knowledge_card(card_id, title, content)
        │
        ├─[同步] store.index_card_fts(card_id, title, content)   ← Storage 层,纯 DB
        │
        └─[异步 spawn] EmbeddingClient::embed(title + "\n" + content)
                │
                ▼
        store.index_card_embedding(card_id, vec)
                │
                ▼
        [失败?] warn! + 卡片仍可被 cards_fts 搜到(降级)
```

**关键决策**:
- **FTS 同步、embedding 异步 spawn**——与 message 路径一致;卡片写入虽非高频,但 conference 流程是关键路径,HTTP embed 延迟不应阻塞卡片入库
- **服务层协调**——`ArchiveSystem::index_knowledge_card` 持有 `EmbeddingClient` + `Storage`,负责"FTS sync + embedding async"的编排;Storage trait 本身不感知 HTTP(见 §4.4)
- **embed 文本 = `title + "\n" + content`**——title 是任务描述,提供 query 语义锚点
- `update_knowledge_card` 也要触发 re-index(conference 生成时走 insert 路径;用户手改走 update 路径,两条都调 `ArchiveSystem::index_knowledge_card`)

### 5.3 查询路径(实时,`/api/knowledge/search`)

```
GET /api/knowledge/search?q=xxx&limit=20
        │
        ▼
routes/knowledge.rs::search()            ← 改:调 search_knowledge_hybrid
        │
        ▼
HybridSearcher::search(query, limit)
        │
        ├─[并行 try_join!]──────────────────────────┐
        │                                            │
        ▼                                            ▼
   embed(query) → query_vec            search_messages_fts(query, k=60)
        │                                            │
        ▼                                            ▼
   search_message_vectors(vec, k=60)    search_cards_fts(query, k=60)
        │                                            │
        ▼                                            ▼
   search_card_vectors(vec, k=60)       (FTS5 IO,与 embed 并行)
        │
        └─────────────────┬──────────────────────────┘
                          ▼
                跨表 RRF(k=60)
                          │
                          ▼
               去重 + top-20 (surface,key)
                          │
                          ├─ message keys → fetch_messages_by_keys (批量)
                          └─ card_ids    → fetch_cards_by_ids (批量)
                          │
                          ▼
               Vec<KnowledgeResult + source>
                          │
                          ▼
        HTTP 返回(结构向后兼容,前端读 source 区分)
```

**关键决策**:
- **K = min(limit × 3, 200)**
- **跨表去重**:`(surface, key)` 二元组天然不冲突,RRF 直接累加
- **降级**:embed/vector 失败 → 向量两路空,RRF 退化 FTS5 两路
- **高亮**:message 走 FTS5 带 `<b>`;card 走 FTS5 也带 `<b>`;向量路回表都不带高亮(可接受代价)

### 5.4 重建路径(`POST /api/knowledge/rebuild`)

```
POST /api/knowledge/rebuild
        │
        ▼
ArchiveSystem::rebuild_all()   ← 新,串行 4 步
        │
        ├─[1] rebuild_fts()                    ← 现有 messages FTS
        ├─[2] rebuild_cards_fts()              ← 新
        ├─[3] rebuild_message_vectors()        ← 新,流式分批 embed
        └─[4] rebuild_card_vectors()           ← 新,卡片少,一次性
                │
                ▼ (message 重建:每批 256 条 embed_batch)
        SELECT session_id, seq, content FROM messages WHERE content != ''
                │
                ▼
        EmbeddingClient::embed_batch(batch)  HTTP→LMStudio
                │
                ▼
        批量 INSERT INTO message_embeddings
                │
                ▼
        返回 { indexed_fts, indexed_cards_fts, indexed_message_vectors, indexed_card_vectors }
```

**关键决策**:
- **分批 256**(messages),卡片数量少一次性 embed_batch
- **单事务批插**
- **响应结构扩展**:4 个计数,前端按需展示
- **不并发**:重建是重 HTTP 任务(LMStudio 单卡),串行最稳

### 5.5 配置开关(`config/default.yaml`)

```yaml
vector_search:
  enabled: false
  embedding:
    base_url: "http://localhost:1234"   # LMStudio;也支持 vLLM/Ollama OpenAI 兼容
    api_key: null                        # LMStudio 通常不需要
    model: "bge-m3"                      # LMStudio 里 pull 的模型名(BAAI/bge-m3)
    timeout_secs: 10
    batch_size: 256
  rebuild_on_startup: false
```

**`enabled: false` 时**:不构造 EmbeddingClient,不触发任何向量路径,行为跟现在 100% 一致。

---

## 6. 错误处理

### 6.1 失败矩阵

| # | 失败点 | 影响 | 行为 | 日志 |
|---|--------|------|------|------|
| 1 | 启动 sqlite-vec load 失败 | 向量整体不可用 | 运行期 `enabled=false`,服务正常起 | `error!` + banner `[vector: DISABLED]` |
| 2 | 启动 `probe_dim` HTTP 失败(LMStudio 没起/模型没 pull) | 同上 | 同上;打印 "请在 LMStudio pull bge-m3" 指引 | `error!` |
| 3 | message 写入 embed 失败 | 单条无向量 | 丢弃,FTS5 仍索引 | `warn!`(限频) |
| 4 | card 写入 embed 失败 | 卡片无向量 | 卡片仍可被 cards_fts 搜到 | `warn!` |
| 5 | 查询 embed 失败 | 该次查询退化 FTS5 两路 | 向量两路空,RRF 降级 | `warn!` |
| 6 | 任一 search_* DB 失败 | 该路空 | 其余继续 | `warn!` |
| 7 | 全部 4 路失败 | 查询彻底失败 | `Err` → HTTP 500 | `error!` |
| 8 | 重建某批 embed 失败 | 该批跳过 | 计入 failed 计数,不中断 | `warn!` |
| 9 | `enabled=false` | 全部向量路径不触发 | 等价当前行为 | 无 |

**核心原则**:**FTS5 是底线,向量是增益**。最坏退化纯 FTS5。

**warn! 限频**:连续失败每 100 条只打一条,带"已抑制 N 条"。

### 6.2 `FoundationError` 扩展(`foundation/src/error.rs`)

```rust
pub enum FoundationError {
    // ... 现有

    #[error("sqlite-vec extension not available: {0}")]
    VectorExtensionUnavailable(String),

    #[error("embedding service unavailable at {url}: {detail}")]
    EmbeddingServiceUnavailable { url: String, detail: String },

    #[error("embedding inference failed: {0}")]
    EmbeddingInferenceFailed(String),

    #[error("vector dimension mismatch: expected {expected}, got {got}")]
    VectorDimensionMismatch { expected: usize, got: usize },
}
```

**废弃** `VectorSearchError`(现有枚举,无生产用例)。

### 6.3 启动顺序约定

配置加载后,若 `vector_search.enabled=true`:
1. **先 load sqlite-vec 扩展** → 失败则运行期 `enabled=false`
2. **再 `EmbeddingClient::probe_dim()`** embed("ping") → 失败(连不上/模型没 pull)则 `enabled=false`
3. 两步都成功 → 用探测维度建两张 vec0 表(BGE-M3 预期 1024),`enabled=true`

配置值不变,只是运行期降级。启动 banner 标注 `[vector: ENABLED dim=1024]` 或 `[vector: DISABLED reason=...]`。

---

## 7. 测试策略

### 7.1 层 1:单元测试(纯函数,秒级)

- **`hybrid_search.rs::rrf_fuse_cross_table`**:
  - 4 路全命中 → 分数正确累加
  - 向量两路全空 → 退化 FTS5 两路排序
  - 单路空 → 其余继续
  - 跨表 key 不冲突(message "session|seq" vs card "uuid")
  - limit 截断
- **`sqlite.rs::cjk_tokenize`** — 现有,保留

### 7.2 层 2:集成测试(SQLite + vec0 + mock embedding,秒级)

`tests/hybrid_search_integration.rs` —— 真 SQLite + vec0,embedding 用 **mock**(`MockEmbeddingClient`,HTTP 不实际调用,返回确定性伪向量)。

测什么:
- 写 5 条 message + 2 张 card → `message_embeddings` 5 行 + `card_embeddings` 2 行 + `cards_fts` 2 行
- 跨表查询 → 返回混合结果,`source` 字段正确区分 message/card
- embed mock 注入失败 → 退化 FTS5 两路(用必命中的关键词验证)
- message vector 失败 → card 路继续
- 重建 → 4 个计数正确
- 卡片 update → card_embeddings 重新 embed

**为什么 mock**:真 LMStudio + BGE-M3 要 ~2-3GB VRAM,CI 跑不起;测试关心**融合逻辑 + 降级**,不是 embedding 质量。

### 7.3 层 3:真模型 smoke 测试(手动,`#[ignore]`)

`tests/smoke_real_model.rs`,本地 LMStudio 在跑时 `cargo test -- --ignored`:

- `embed("如何让 AI 记住我")` 与 `embed("长期记忆怎么实现")` cosine > 0.6
- `embed("如何让 AI 记住我")` 与 `embed("今天天气真好")` cosine < 0.3
- 端到端:写 message + card,hybrid 查询语义召回(同义改述)
- 卡片内容"意识哲学的困境" → 搜 "心灵哲学问题" 能命中

**不进 CI**。

---

## 8. 验收标准(DoD)

1. `cargo test` 全绿(层 1 + 层 2)
2. `cargo build --release` 通过
3. `vector_search.enabled=false` 时所有现有测试 + 行为不变(零回归)
4. `vector_search.enabled=true` + LMStudio 在跑 + bge-m3 已 pull:
   - 写 message → `message_embeddings` 新增一行
   - conference 生成 card → `card_embeddings` + `cards_fts` 新增一行
   - `/knowledge/search` 返回混合 message + card 结果,带 `source` 字段
   - 故意停 LMStudio → 搜索降级纯 FTS5,不报错
5. 重建 API 返回 4 个计数
6. smoke 测试手动跑过(语义召回 + 跨表混合验证)

---

## 9. 文件改动清单

| 文件 | 改动 |
|------|------|
| `rust/foundation/src/embedding.rs` | 新建 — HTTP EmbeddingClient(非 ONNX) |
| `rust/foundation/src/hybrid_search.rs` | 新建 — 跨表 RRF + 降级 |
| `rust/foundation/src/sqlite.rs` | 加 2 张 vec0 表 + cards_fts 表;加 KNN/FTS/重建/回表方法;现有 `search_knowledge` 重构出 `search_messages_fts` |
| `rust/foundation/src/storage.rs` | trait 加 `search_knowledge_hybrid` / `index_knowledge_card` |
| `rust/foundation/src/lib.rs` | 导出新模块 |
| `rust/foundation/src/config.rs` | 加 `VectorSearchConfig`(base_url/api_key/model/timeout/batch_size) |
| `rust/foundation/src/models.rs` | `KnowledgeResult` 加 `source` / `card_id` / `tags` 字段 |
| `rust/foundation/src/error.rs` | 加 4 个向量 error variant;废弃 `VectorSearchError` |
| `rust/foundation/Cargo.toml` | 加 `reqwest`(若未在 workspace);**不加** `ort`/`tokenizers` |
| `rust/foundation/extensions/` | 新建,放 sqlite-vec `.dylib`/`.so`/`.dll` |
| `rust/api/src/store.rs` | impl `search_knowledge_hybrid` / `index_knowledge_card`;注入 `HybridSearcher` |
| `rust/api/src/state.rs` | `AppState` 持有 `Option<Arc<HybridSearcher>>` |
| `rust/api/src/main.rs` | 启动按配置条件构造 client + searcher + probe_dim |
| `rust/api/src/routes/knowledge.rs` | `search` 改调 `search_knowledge_hybrid`;`rebuild` 改调 4 步重建 |
| `rust/archive/src/lib.rs` | 加 `search_knowledge_hybrid` / `index_knowledge_card` 转发 |
| `rust/possession/src/modes/conference.rs` | `insert_knowledge_card` 后调 `index_knowledge_card`(FTS+embed) |
| `config/default.yaml` | 加 `vector_search` 配置块 |
| `nextjs/components/knowledge-browser.tsx` | 读 `source` 字段区分 message/card 显示（可选,后端先兼容，前端已迁移至 TanStack Start） |

---

## 10. 风险与边界

- **LMStudio 必须在跑**——向量功能依赖外部服务;但项目本来就把 LMStudio 当 LLM 后端,不算新增依赖
- **BGE-M3 资源占用**——568M 模型,LMStudio 加载约需 ~2-3GB VRAM(FP16)或可纯 CPU 跑(~200ms/条);远低于 Qwen3-4B 的 8-9GB,消费级机器友好。8192 token 上下文足以覆盖绝大多数 message,超长内容由 LMStudio 截断
- **首次启用必须 rebuild**——历史 messages/cards 无向量,启用后调 `/knowledge/rebuild` 全量灌库。rebuild 1w messages + LMStudio BGE-M3(GPU)≈ 5-15 分钟,(CPU)≈ 30-60 分钟,需避开高峰
- **sqlite-vec 跨平台分发**——`.dylib`/`.so`/`.dll` 随仓库,writing-plans 单独验证 CI 能 load
- **WAL + vec0 并发**——同库单写者;message 异步 spawn 串行,card 同步在 conference 流程内,不抢锁
- **embedding 模型升级**——维度变(如换 Qwen3-4B 或调输出维度)→ drop 两张 vec0 表 + rebuild。schema 用 `probe_dim` 探测值,不硬编码,降低此风险
- **HTTP 延迟**——LMStudio loopback embed,BGE-M3 GPU ~50-100ms / CPU ~200ms,查询路径并行掩盖;message 写入异步掩盖;均可接受
- **BGE-M3 三模式未充分利用**(未来机会)——当前只用 dense 模式;BGE-M3 原生支持 sparse(词法)+ multi-vector(ColBERT),未来可探索用 sparse 替代/补充 FTS5,或用 multi-vector 提升长文档召回。本 spec 不实施,留作 v3 演进方向
- **向后兼容**——`KnowledgeResult` 新字段 serde 默认值,旧前端不读 source 也能正常显示 message 结果

---

## 附录 A:版本变更摘要

### v1 → v2

| 维度 | v1 | v2 |
|------|----|----|
| embedding 后端 | 进程内 ONNX(bge-small-zh) | **LMStudio HTTP(Qwen3-4B)** |
| 重依赖 | +`ort` +`tokenizers` | **无新增**(reqwest 已用) |
| 模型分发 | 仓库塞 100MB ONNX + 下载脚本 | **不塞**,LMStudio 自管 |
| 索引范围 | messages only | **messages + KnowledgeCard** |
| FTS5 表 | 1 张(knowledge_fts) | **2 张**(+ cards_fts) |
| vec0 表 | 1 张 | **2 张**(message/card 各一) |
| 查询入口 | /search 只搜 messages | **/search 跨表 RRF** |
| 返回结构 | KnowledgeResult | **+ source / card_id / tags** |
| 重建计数 | 2 个(fts/vector) | **4 个**(msg_fts/card_fts/msg_vec/card_vec) |
| 融合函数 | rrf_fuse(2 路) | **rrf_fuse_cross_table(4 路)** |
| RRF 去重 key | (session_id, seq) | **(surface, key) 二元组** |

### v2 → v2.1(本次)

| 维度 | v2 | v2.1 |
|------|----|----|
| embedding 模型 | Qwen3-Embedding-4B | **BGE-M3** |
| 维度 | 2560 | **1024** |
| 最大上下文 | 32768 token | 8192 token(对 message 场景足够) |
| VRAM 占用 | 8-9GB(FP16) | **~2-3GB(FP16)/可纯 CPU** |
| 多语言 | 多语言 | **100+ 语言,长期中文霸榜** |
| 检索模式 | dense only | **dense + sparse + multi-vector(本版仅用 dense)** |
| 架构改动 | — | **无**(只改 model 名 + 维度,代码主体不变) |

**v2.1 净收益**:部署更灵活(进程内/HTTP 双可行)、资源占用更低、未来有 sparse/multi-vector 演进空间,代价是 benchmark 上略逊 Qwen3-4B 几个点(用户无感)。
