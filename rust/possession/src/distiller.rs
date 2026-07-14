use std::sync::Arc;
use std::time::Duration;

use ai_gateway::GatewayRegistry;
use chrono::Utc;
use foundation::{
    CallConfig, LLMRequest, ObservationType, Prompt, PromptMessage, Provider, SessionObservation,
    Storage,
};
use crate::{WsEvent, WsEventType, WsSessionManager};

const DISTILL_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DigestSummary {
    pub summary: String,
    pub observation_count: usize,
}

#[derive(Debug, serde::Deserialize)]
struct ParsedDigest {
    summary: String,
    observations: Vec<ParsedObservation>,
}

#[derive(Debug, serde::Deserialize)]
struct ParsedObservation {
    #[serde(rename = "type")]
    obs_type: String,
    title: String,
    content: String,
    soul: Option<String>,
    seq: Option<i64>,
    confidence: Option<f32>,
}

const SYSTEM_PROMPT: &str = r#"你是 Snake Skin 的记忆压缩器。你的任务是从一次"魂合议"对话中提取 5-10 条原子级知识点（observation），并给出一句整体总结。

**你必须只输出纯 JSON。不要输出思考过程、不要输出分析步骤、不要输出任何 JSON 之外的文字。**

## observation 类型定义
每条 observation 必须归入以下 8 类之一：
- session: 🎯 整体回顾/元信息
- discovery: 🔵 新发现/新认知
- decision: ⚖️ 做出的决策或立场
- bugfix: 🔴 发现的问题或修正
- feature: 🟣 新能力或新方案
- refactor: 🔄 重组/重新框架化
- change: ✅ 一般性变更/事实
- security: 🚨 安全或边界问题

## 输出格式示例（严格按这个格式输出，不要 markdown 代码块）

```json
{
  "summary": "用户通过魂合议分析了数据采集泛化对劳动者的影响，核心矛盾是门槛降低导致议价权丧失",
  "observations": [
    {
      "type": "discovery",
      "title": "专用设备壁垒被通用终端瓦解",
      "content": "祝鹤槐指出工业相机一套三十万的壁垒被手机+AI眼镜替代，这是技术民主化的必然代价。朋友在深圳做工业相机的生意直接受冲击。",
      "soul": "祝鹤槐",
      "seq": 3,
      "confidence": 0.95
    },
    {
      "type": "decision",
      "title": "不追采集追判断",
      "content": "祝鹤槐建议劳动者不要学采集技能（即将被自动化），要学数据标注和数据治理技能。3D点云标注工人月入两万，判断层的工作还不能完全自动化。",
      "soul": "祝鹤槐",
      "seq": 3,
      "confidence": 0.9
    }
  ]
}
```

要求：
1. 每条 observation 必须有独立的认知价值，不重复
2. title 简洁（≤30字）、content 具体（≤200字）
3. type 必须是上述 8 类之一
4. soul 和 seq 对应对话中产出该内容的魂和消息序号
5. confidence 表示可靠程度 (0.0-1.0)
6. summary ≤100字
7. **绝对禁止输出 JSON 以外的任何文字**"#;

/// Compress a completed session into observations + digest summary.
pub async fn distill_session(
    store: Arc<dyn Storage>,
    gateway: Arc<GatewayRegistry>,
    session_id: &str,
) -> foundation::error::Result<DigestSummary> {
    let session = store.get_session(session_id).await?;
    let messages = store.get_messages(session_id).await?;

    if messages.is_empty() {
        return Ok(DigestSummary {
            summary: "空会话".to_string(),
            observation_count: 0,
        });
    }

    // Build conversation text for the prompt (truncate to ~8000 bytes, UTF-8 safe)
    let conversation = build_conversation_text(&messages, &session.title);
    let truncated = if conversation.len() > 8000 {
        let boundary = conversation
            .char_indices()
            .take_while(|(i, _)| *i < 8000)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...(共 {} 字符，已截断)", &conversation[..boundary], conversation.len())
    } else {
        conversation
    };

    let user_msg = format!(
        "会话标题: {}\n模式: {}\n\n对话内容:\n{}\n\n---\n现在输出 JSON。只输出 JSON，不要任何其他文字。",
        session.title, session.mode.as_str(), truncated
    );

    let prompt = Prompt {
        messages: vec![
            PromptMessage {
                role: "system".into(),
                content: SYSTEM_PROMPT.to_string(),
                reasoning_content: None,
                ..Default::default()
            },
            PromptMessage {
                role: "user".into(),
                content: user_msg,
                reasoning_content: None,
                ..Default::default()
            },
        ],
    };

    // 自动选择可用 provider
    let provider = gateway.pick_provider().unwrap_or(Provider::DeepSeek);
    tracing::info!("distill using provider: {:?}", provider);

    // DeepSeek 不支持 structured output (response_format)，跳过
    let use_structured = !matches!(provider, Provider::DeepSeek);

    let config = CallConfig {
        temperature: 0.3,
        max_tokens: 4096,
        stream: false,
        model: None,
        thinking_enabled: Some(false),
        structured_output: if use_structured {
            Some(foundation::StructuredOutputConfig {
                enabled: true,
                json_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "summary": { "type": "string" },
                        "observations": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": { "type": "string", "enum": ["session", "discovery", "decision", "bugfix", "feature", "refactor", "change", "security"] },
                                    "title": { "type": "string" },
                                    "content": { "type": "string" },
                                    "soul": { "type": ["string", "null"] },
                                    "seq": { "type": ["integer", "null"] },
                                    "confidence": { "type": ["number", "null"] }
                                },
                                "required": ["type", "title", "content"]
                            }
                        }
                    },
                    "required": ["summary", "observations"]
                })),
            })
        } else {
            None
        },
        ..Default::default()
    };

    let req = LLMRequest {
        provider,
        prompt,
        config,
    };

    let mut rx = gateway.call(&req).map_err(|e| {
        foundation::error::FoundationError::InvalidState(format!("distill call failed: {}", e))
    })?;

    let mut raw = String::new();
    let mut distill_usage = foundation::UsageStats::default();
    while let Some(r) = rx.recv().await {
        match r {
            Ok(c) => {
                if let Some(u) = c.usage { distill_usage = u; }
                raw.push_str(&c.content);
            }
            Err(e) => {
                tracing::warn!("distill chunk error: {}", e);
                break;
            }
        }
    }

    if raw.is_empty() {
        tracing::warn!("distill empty response for session {}", session_id);
        // 即使失败也写入 summary，防止每次打开页面重复触发 distill
        let _ = store.update_session_digest(session_id, "压缩失败：LLM 无输出").await;
        return Ok(DigestSummary {
            summary: "压缩失败：LLM 无输出".to_string(),
            observation_count: 0,
        });
    }

    let raw_preview: String = raw.chars().take(500).collect();
    tracing::info!("distill raw response (first 500 chars): {}", raw_preview);

    // Strip thinking process and markdown code fences
    let after_thinking = strip_thinking(&raw);
    let json_str = strip_code_fence(after_thinking);

    let parsed: ParsedDigest = match serde_json::from_str::<ParsedDigest>(json_str) {
        Ok(p) => {
            tracing::info!("distill parsed ok: {} observations", p.observations.len());
            p
        }
        Err(e) => {
            tracing::warn!("distill parse error: {} | raw: {}", e, raw_preview);
            // 即使解析失败也写入 summary，防止重复触发
            let _ = store.update_session_digest(session_id, &format!("压缩解析失败: {}", e)).await;
            return Ok(DigestSummary {
                summary: format!("压缩解析失败: {}", e),
                observation_count: 0,
            });
        }
    };

    // Estimate tokens: rough heuristic ~1.5 tokens per CJK char, ~0.75 per ASCII char
    let estimate_tokens = |text: &str| -> u32 {
        let cjk = text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)).count() as u32;
        let other = text.len() as u32 / 2;
        (cjk * 2 + other) / 3
    };

    let work_tokens: u32 = messages.iter().map(|m| estimate_tokens(&m.content)).sum();
    let now = Utc::now();

    let rows: Vec<SessionObservation> = parsed
        .observations
        .iter()
        .map(|o| SessionObservation {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            soul_name: o.soul.clone(),
            obs_type: ObservationType::from_str(&o.obs_type).unwrap_or(ObservationType::Discovery),
            title: o.title.clone(),
            content: o.content.clone(),
            source_seq: o.seq,
            read_tokens: estimate_tokens(&o.content),
            work_tokens: (work_tokens as f32 / parsed.observations.len().max(1) as f32) as u32,
            confidence: o.confidence.unwrap_or(0.7).clamp(0.0, 1.0),
            created_at: now,
        })
        .collect();

    let count = rows.len();
    store.insert_session_observations(&rows).await?;
    store.update_session_digest(session_id, &parsed.summary).await?;

    if distill_usage.total_tokens > 0 {
        let summary_trunc = if parsed.summary.len() > 80 {
            let boundary = parsed.summary.char_indices()
                .take_while(|(i, c)| *i + c.len_utf8() <= 80)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            if boundary > 0 {
                &parsed.summary[..boundary]
            } else {
                ""
            }
        } else {
            &parsed.summary
        };
        let _ = store.record_call(&foundation::CallRecord {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            soul_name: "蒸馏官".to_string(),
            mode: foundation::PossessionMode::Conference,
            task_summary: format!("distill: {}", summary_trunc),
            effectiveness: foundation::Effectiveness::Effective,
            notes: "[distill]".to_string(),
            created_at: Utc::now(),
            self_negation: None,
            empty_chair: None,
            user_feedback: None,
            usage: distill_usage,
        }).await;
    }

    Ok(DigestSummary {
        summary: parsed.summary,
        observation_count: count,
    })
}

fn build_conversation_text(messages: &[foundation::Message], _title: &str) -> String {
    use foundation::MessageRole;
    let mut out = String::new();
    for msg in messages {
        let role_label = match msg.role {
            MessageRole::User => "👤用户",
            MessageRole::Soul => {
                let name = msg.soul_name.as_deref().unwrap_or("魂");
                // Use a generic label to avoid emoji in code
                &*format!("🎭{}", name)
            }
            MessageRole::Synthesis => "🧠综合",
            MessageRole::System => "⚙️系统",
        };
        // Truncate individual messages to ~1500 chars (respect UTF-8 boundaries)
        let content = if msg.content.len() > 1500 {
            let boundary = msg.content.char_indices()
                .take_while(|(i, _)| *i < 1500)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(1500);
            format!("{}...(截断)", &msg.content[..boundary])
        } else {
            msg.content.clone()
        };
        out.push_str(&format!("[seq={}] {}: {}\n\n", msg.seq, role_label, content));
    }
    out
}

/// 过滤 LLM thinking / reasoning 内容，只保留 JSON 部分
fn strip_thinking(s: &str) -> &str {
    let trimmed = s.trim_start();

    // 检测常见的 thinking 前缀（不区分大小写，覆盖更多变体）
    let lower = trimmed.to_lowercase();
    let has_thinking = lower.starts_with("here's a thinking process:")
        || lower.starts_with("<thinking>")
        || lower.starts_with("<think>")
        || lower.starts_with("<reasoning>")
        || lower.starts_with("reasoning:")
        || lower.starts_with("analysis:")
        || lower.starts_with("thinking:")
        || lower.contains("\nthinking:");

    if !has_thinking {
        return s;
    }

    // 策略1：找最靠后的 JSON 对象开始位置（避免 thinking 过程中包含 { 的干扰）
    // 匹配 {"summary"、{ "summary"、{\n  "summary" 等各种空白变体
    let mut best_idx: Option<usize> = None;
    for marker in &[
        "{\"summary\"",
        "{ \"summary\"",
        "{\n\"summary\"",
        "{\n \"summary\"",
        "{\n  \"summary\"",
        "{\n   \"summary\"",
        "{\n    \"summary\"",
        "{\r\n\"summary\"",
        "{\r\n \"summary\"",
        "{\r\n  \"summary\"",
        "{\r\n    \"summary\"",
    ] {
        if let Some(idx) = s.rfind(marker) {
            best_idx = Some(best_idx.map_or(idx, |b: usize| b.max(idx)));
        }
    }

    if let Some(idx) = best_idx {
        return &s[idx..];
    }

    // 策略2：从末尾找最后一个包含 "summary" 键的 JSON 对象开头
    // 先找到最后一个 "summary" 出现的位置，然后向前找最近的 '{'
    if let Some(summary_pos) = s.rfind("\"summary\"") {
        let prefix = &s[..summary_pos];
        // 向前扫描找最近的 '{'（跳过中间的非 JSON 字符）
        if let Some(brace_pos) = prefix.rfind('{') {
            let candidate = s[brace_pos..].trim();
            if candidate.starts_with('{') {
                return &s[brace_pos..];
            }
        }
    }

    // 策略3：从字符串末尾往回找，找最后一个看起来是 JSON 开头的 '{'
    // 跳过末尾的空白和代码块标记
    let trimmed_end = s.trim_end();
    let end_pos = if trimmed_end.ends_with("```") {
        trimmed_end.len() - 3
    } else {
        trimmed_end.len()
    };

    // 找最后一个 '{'，它后面应该跟着 '"' 或 '\n'
    if let Some(idx) = s[..end_pos].rfind('{') {
        let after = s[idx..end_pos].trim_start();
        if after.starts_with('{') {
            return &s[idx..];
        }
    }

    // 策略4：fallback 到第一个 '{'
    if let Some(idx) = s.find('{') {
        return &s[idx..];
    }

    s
}

fn strip_code_fence(s: &str) -> &str {
    let trimmed = s.trim();

    // 完整的 ```json ... ``` 包裹
    if (trimmed.starts_with("```json") || trimmed.starts_with("```")) && trimmed.ends_with("```") {
        let start = trimmed.find('\n').map(|i| i + 1).unwrap_or(7);
        let end = trimmed.len() - 3;
        if start < end {
            return &trimmed[start..end];
        }
    }

    // 只在结尾有 ``` 标记（strip_thinking 后残留）
    if let Some(stripped) = trimmed.strip_suffix("```") {
        let s = stripped.trim();
        if s.starts_with('{') {
            return s;
        }
    }

    s
}

/// Run distill in background and notify via WebSocket on completion.
pub fn spawn_distill(
    store: Arc<dyn Storage>,
    gateway: Arc<GatewayRegistry>,
    ws: WsSessionManager,
    session_id: String,
) {
    tokio::spawn(async move {
        tracing::info!("distill started for session {}", session_id);
        let timeout = Duration::from_secs(DISTILL_TIMEOUT_SECS);
        // Clone store for fallback writes in error/timeout branches
        let store_for_fallback = store.clone();
        match tokio::time::timeout(timeout, distill_session(store, gateway, &session_id)).await {
            Ok(Ok(digest)) => {
                tracing::info!(
                    "distill completed for session {}: {} observations",
                    session_id,
                    digest.observation_count
                );
                let _ = ws.broadcast_system(
                    &session_id,
                    &WsEvent {
                        event_type: WsEventType::ObservationsReady,
                        payload: serde_json::to_string(&digest).unwrap_or_default(),
                        reasoning_content: None,
                        soul_name: None,
                        seq: 0,
                    },
                );
            }
            Ok(Err(e)) => {
                tracing::warn!("distill failed for session {}: {}", session_id, e);
                let _ = store_for_fallback.update_session_digest(&session_id, "压缩失败：引擎错误").await;
            }
            Err(_) => {
                tracing::warn!("distill timed out after {}s for session {}", DISTILL_TIMEOUT_SECS, session_id);
                let _ = store_for_fallback.update_session_digest(&session_id, "压缩失败：超时").await;
            }
        }
    });
}
