use std::time::Duration;

use ai_gateway::prompt::PromptBuilder;
use ai_gateway::GatewayRegistry;
use foundation::{
    CallConfig, KnowledgeCard, LLMRequest, Message, Prompt, PromptMessage, ReasoningEffort, Result, Storage,
};
use registry::SoulRegistry;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use crate::cross_detector::CrossDetector;

use crate::stream;
use crate::tools::ToolRegistry;
use crate::{SoulOutput, UserPresets, WsEvent, WsEventType, WsSessionManager};
use super::topology;

const MAX_PARALLEL_SOULS: usize = 10;
const SOUL_TIMEOUT_SECS: u64 = 180;

/// 按 UTF-8 字符边界安全截断字符串到不超过 `max_bytes` 字节。
/// 避免直接 `&s[..n]` 切到多字节字符中间导致 panic。
fn truncate_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // floor_char_boundary 在 1.82 才稳定，这里用 char_indices 手动回退到上一个边界
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// 增强的合议模式
pub async fn run(
    store: &dyn Storage,
    registry: &SoulRegistry,
    gateway: &GatewayRegistry,
    ws: &WsSessionManager,
    session_id: &str,
    task: &str,
    souls: &[String],
    task_cards: &std::collections::HashMap<String, String>,
    presets: &UserPresets,
    system_tx: &mpsc::Sender<WsEvent>,
    tool_registry: &ToolRegistry,
    domain: &foundation::DomainProfile,
) -> Result<Vec<SoulOutput>> {
    let limited: Vec<String> = souls.iter().take(MAX_PARALLEL_SOULS).cloned().collect();
    let _providers = gateway.list_providers();
    let prompt_builder = PromptBuilder::with_domain(domain.clone());
    let task_arc = std::sync::Arc::new(task.to_string());

    // 创建交叉检测器（仅收集碰撞信息供 synthesis prompt 使用）
    let cross_detector = CrossDetector::new();

    let mut requests: Vec<(String, LLMRequest)> = Vec::with_capacity(limited.len());
    let mut profiles: Vec<foundation::SoulProfile> = Vec::with_capacity(limited.len());
    for soul_name in &limited {
        let _ = system_tx.try_send(WsEvent {
            event_type: WsEventType::SoulStarted,
            payload: format!("正在召唤 {} ...", soul_name),
            reasoning_content: None,
            soul_name: Some(soul_name.clone()),
            seq: 0,
        }).ok();

        // 注册魂到检测器
        cross_detector.register_soul(soul_name.clone());

        match registry.get_soul(soul_name) {
            Ok(profile) => {
                // 使用用户配置的 provider，不再硬编码 DeepSeek 优先
                let provider = gateway.pick_provider().unwrap_or(foundation::Provider::DeepSeek);
                let mut config = CallConfig::default().with_reasoning_effort(ReasoningEffort::Think);
                let tier = foundation::ModelTier::for_provider(&provider, config.model.as_deref().unwrap_or("unknown"));

                let tool_names = crate::tools::parse_soul_tools(&profile.tools);
                if !tool_names.is_empty() {
                    let definitions = tool_registry.filter_definitions(&tool_names);
                    if !definitions.is_empty() {
                        config = config.with_tools(definitions);
            }
                }

                let prompt = if let Some(card) = task_cards.get(soul_name) {
                    prompt_builder.build_summon_with_task_card(
                        &profile, &task_arc, card,
                        presets.judgment.as_deref(),
                        presets.worry.as_deref(),
                        presets.unknown.as_deref(),
                        tier,
                        presets.search_results.as_deref(),
                        presets.interrogation_context.as_deref(),
                    )
                } else {
                    prompt_builder.build_summon_prompt(
                        &profile, &task_arc,
                        presets.judgment.as_deref(),
                        presets.worry.as_deref(),
                        presets.unknown.as_deref(),
                        tier,
                        presets.search_results.as_deref(),
                        presets.interrogation_context.as_deref(),
                    )
                };
                profiles.push(profile.clone());
                requests.push((soul_name.clone(), LLMRequest {
                    provider, prompt, config,
                }));
            }
            Err(e) => {
                let _ = system_tx.try_send(WsEvent {
                    event_type: WsEventType::SoulError,
                    payload: e.to_string(),
                    reasoning_content: None,
                    soul_name: Some(soul_name.clone()),
                    seq: 0,
                }).ok();
            }
        }
    }

    // ── 拓扑规划：根据任务复杂度和魂多样性选择最优编排策略 ──
    let planner = topology::TopologyPlanner::new();
    let selected_topology = topology::plan_from_profiles(
        &planner, &profiles, task, false,
    );
    let _ = system_tx.try_send(WsEvent {
        event_type: WsEventType::ProcessStep,
        payload: format!(
            "拓扑规划: {} (参与{}魂, 预估{}次LLM调用)",
            selected_topology.describe(),
            selected_topology.soul_count(),
            selected_topology.estimated_calls()
        ),
        reasoning_content: None,
        soul_name: None,
        seq: 0,
    }).ok();

    let gateway_owned = GatewayRegistry::clone(gateway);
    let _ws_c = ws.clone();
    let _s_id = session_id.to_string();
    let mut set = JoinSet::new();

    for (soul_name, req) in requests {
        let s_id = session_id.to_string();
        let ws_c = ws.clone();
        let gw = gateway_owned.clone();
        let tr = tool_registry.clone();
        let provider = req.provider.clone();
        let prompt = req.prompt.clone();
        let config = req.config.clone();
        let detector = cross_detector.clone();
        set.spawn(async move {
            run_soul_with_tools(
                &gw, &provider, &prompt, &config,
                &s_id, &soul_name, &ws_c, &tr,
                detector,
            ).await
        });
    }

    let expected_soul_count = limited.len();
    let collect = async {
        let mut acc = Vec::with_capacity(expected_soul_count);
        while let Some(r) = set.join_next().await {
            match r {
                Ok(output) => {
                    acc.push(output);
                    if acc.len() >= expected_soul_count {
                        break;
            }
                }
                Err(e) => {
                    acc.push(SoulOutput::error("unknown".into(), e.to_string()));
                }
            }
        }
        acc
    };

    let mut outputs = match tokio::time::timeout(Duration::from_secs(SOUL_TIMEOUT_SECS), collect).await {
        Ok(acc) => {
            set.abort_all();
            acc
        }
        Err(_) => {
            tracing::warn!("Conference timed out after {}s, collecting completed results", SOUL_TIMEOUT_SECS);
            let mut acc = Vec::new();
            loop {
                match tokio::time::timeout(Duration::from_millis(50), set.join_next()).await {
                    Ok(Some(Ok(output))) => {
                        acc.push(output);
            }
                    _ => break,
                }
            }
            set.abort_all();
            let _ = system_tx.send(WsEvent {
                event_type: WsEventType::SystemMessage,
                payload: format!("⚠️ 合议超时（{}秒），已保留 {}/{} 个魂的回应", SOUL_TIMEOUT_SECS, acc.len(), limited.len()),
                reasoning_content: None,
                soul_name: None,
                seq: 0,
            });
            acc
        }
    };

    for output in &outputs {
        crate::emit_soul_cost(system_tx, &output.soul_name, &output.usage, None);
        if let Err(e) = crate::finalize_output(store, session_id, output, foundation::PossessionMode::Conference, task).await {
            tracing::error!("Failed to finalize {} output: {}", output.soul_name, e);
        }
    }

    // ── 多轮碰撞注入（CrossDetector → Soul 第二轮/第三轮回应）──
    let max_rounds: u32 = 3;
    let mut prev_collisions: Vec<crate::cross_detector::CollisionEvent> = vec![];

    for round in 1..max_rounds {
        // 以所有已产生的输出为检测源
        let outputs_for_detection = &outputs;
        let mut detector = CrossDetector::new();
        for o in outputs_for_detection {
            detector.register_soul(o.soul_name.clone());
            if !o.content.is_empty() {
                // 把完整输出喂入检测器
                for token in o.content.split_inclusive(|_| true) {
                    detector.add_token(&o.soul_name, token);
                }
            }
        }
        let new_collisions = detector.detect_collisions();

        // 过滤掉和上一轮重复的碰撞
        let fresh_collisions: Vec<_> = new_collisions.iter()
            .filter(|c| !prev_collisions.iter().any(|pc|
                pc.from_soul == c.from_soul && pc.to_soul == c.to_soul && pc.collision_type == c.collision_type
            ))
            .cloned()
            .collect();

        if fresh_collisions.is_empty() {
            tracing::info!("Round {}: no new collisions, stopping multi-round", round + 1);
            break;
        }

        tracing::info!(
            "Round {}: {} new collisions detected, injecting into soul prompts",
            round + 1,
            fresh_collisions.len()
        );

        // 构建每个 Soul 的碰撞上下文
        let mut collision_context: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for c in &fresh_collisions {
            collision_context.entry(c.to_soul.clone())
                .or_default()
                .push(format!("**{}** 说：{}", c.from_soul, c.content));
        }

        // 为被碰撞的 Soul 构建第二轮 prompt
        let mut round_requests: Vec<(String, LLMRequest)> = vec![];
        let output_map: std::collections::HashMap<String, &SoulOutput> = outputs.iter()
            .map(|o| (o.soul_name.clone(), o))
            .collect();

        for (soul_name, contexts) in &collision_context {
            if let Some(output) = output_map.get(soul_name) {
                if let Ok(profile) = registry.get_soul(soul_name) {
                    let provider = gateway.pick_provider().unwrap_or(foundation::Provider::DeepSeek);
                    let config = CallConfig::default().with_reasoning_effort(ReasoningEffort::Think);
                    let tier = foundation::ModelTier::for_provider(&provider, config.model.as_deref().unwrap_or("unknown"));

                    // 附加碰撞上下文到原始 prompt
                    let base_prompt = prompt_builder.build_summon_prompt(
                        &profile, &task_arc,
                        presets.judgment.as_deref(),
                        presets.worry.as_deref(),
                        presets.unknown.as_deref(),
                        tier,
                        presets.search_results.as_deref(),
                        presets.interrogation_context.as_deref(),
                    );
                    let mut messages = base_prompt.messages.clone();

                    let context_str = contexts.join("\n\n");
                    let injection = format!(
                        "\n\n---\n## ⚡ 其他角色的交叉质证\n\n{}\n\n请你针对以下观点做出回应——特别是你不同意或需要补充的部分：\n",
                        context_str
                    );
                    messages.push(PromptMessage {
                        role: "user".into(),
                        content: injection,
                        reasoning_content: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });

                    let _ = system_tx.try_send(WsEvent {
                        event_type: WsEventType::ProcessStep,
                        payload: format!("⚡ {} 收到来自 {:#?} 的交叉质证，正在生成回应...", soul_name, contexts.len()),
                        reasoning_content: None,
                        soul_name: Some(soul_name.clone()),
                        seq: 0,
                    }).ok();

                    round_requests.push((soul_name.clone(), LLMRequest {
                        provider: provider.clone(),
                        prompt: Prompt { messages },
                        config: config.clone(),
                    }));
                }
            }
        }

        if round_requests.is_empty() {
            break;
        }

        // 并行执行第二轮
        let mut round_set = JoinSet::new();
        for (soul_name, req) in round_requests {
            let s_id = session_id.to_string();
            let ws_c = ws.clone();
            let gw = GatewayRegistry::clone(gateway);
            let tr = tool_registry.clone();
            let provider = req.provider.clone();
            let prompt = req.prompt.clone();
            let config = req.config.clone();
            let detector = CrossDetector::new();
            round_set.spawn(async move {
                run_soul_with_tools(
                    &gw, &provider, &prompt, &config,
                    &s_id, &soul_name, &ws_c, &tr,
                    detector,
                ).await
            });
        }

        let expected = collision_context.len();
        let round_collect = async {
            let mut acc = Vec::with_capacity(expected);
            while let Some(r) = round_set.join_next().await {
                match r {
                    Ok(output) => {
                        acc.push(output);
                        if acc.len() >= expected { break; }
                    }
                    Err(e) => {
                        acc.push(SoulOutput::error("unknown".into(), e.to_string()));
                    }
                }
            }
            acc
        };

        let round_outputs = match tokio::time::timeout(Duration::from_secs(SOUL_TIMEOUT_SECS), round_collect).await {
            Ok(acc) => { round_set.abort_all(); acc }
            Err(_) => {
                let mut acc = Vec::new();
                while let Ok(Some(Ok(output))) = tokio::time::timeout(Duration::from_millis(50), round_set.join_next()).await {
                    acc.push(output);
                }
                round_set.abort_all();
                acc
            }
        };

        for o in &round_outputs {
            crate::emit_soul_cost(system_tx, &o.soul_name, &o.usage, None);
            let _ = crate::finalize_output(store, session_id, o, foundation::PossessionMode::Conference, task).await;
        }

        prev_collisions = new_collisions;
        outputs.extend(round_outputs);
    }
    // ── 多轮碰撞注入结束 ──

    let synthesis_outputs: Vec<(String, String)> = outputs
        .iter()
        .filter(|o| o.error.is_none())
        .map(|o| (o.soul_name.clone(), o.content.clone()))
        .collect();

    if !synthesis_outputs.is_empty() {
        let synthesis_noun = prompt_builder.domain().terms.get("synthesis_noun")
            .cloned().unwrap_or_else(|| "辩证综合".into());
        let _ = system_tx.try_send(WsEvent {
            event_type: WsEventType::SynthesisStarted,
            payload: format!("{}开始...", synthesis_noun),
            reasoning_content: None,
            soul_name: None,
            seq: 0,
        }).ok();

        // 收集碰撞检测结果
        let collisions = cross_detector.get_collisions();
        let collision_summary = if collisions.is_empty() {
            String::new()
        } else {
            let mut summary = String::from("## 检测到的碰撞事件\n\n");
            for c in &collisions {
                summary.push_str(&format!(
                    "- **{}** → **{}**（{:?}）：{}\n",
                    c.from_soul, c.to_soul, c.collision_type, c.content
                ));
            }
            summary.push_str("\n请综合考虑以上碰撞信息——它们可能揭示了各魂在思考过程中真实发生的结构冲突。\n");
            summary
        };

        // 为综合官选择合适的配置
        let synthesis_provider = gateway.pick_provider().unwrap_or(foundation::Provider::DeepSeek);
        let synthesis_config = CallConfig::default().with_reasoning_effort(ReasoningEffort::ThinkMax);
        let card_provider = synthesis_provider.clone();

        let synthesis_prompt = if collision_summary.is_empty() {
            prompt_builder.build_synthesis_prompt(task, &synthesis_outputs)
        } else {
            prompt_builder.build_synthesis_with_collisions(task, &synthesis_outputs, &collision_summary)
        };
        let synthesis_req = LLMRequest { provider: synthesis_provider, prompt: synthesis_prompt, config: synthesis_config };

        if let Ok(rx) = gateway.call(&synthesis_req) {
            if let Ok((content, synth_usage)) = stream::stream_synthesis(rx, session_id, ws).await {
                let per_soul_costs: Vec<(String, foundation::UsageStats, Option<String>)> = outputs.iter()
                    .filter(|o| o.error.is_none())
                    .map(|o| (o.soul_name.clone(), o.usage.clone(), None))
                    .collect();
                crate::emit_session_cost(system_tx, &per_soul_costs, Some(synth_usage.total_tokens));

                if synth_usage.total_tokens > 0 {
                    let _ = store.record_call(&foundation::CallRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: session_id.to_string(),
                        soul_name: "综合官".to_string(),
                        mode: foundation::PossessionMode::Conference,
                        task_summary: task.to_string(),
                        effectiveness: foundation::Effectiveness::Effective,
                        notes: "[synthesis]".to_string(),
                        created_at: chrono::Utc::now(),
                        self_negation: None,
                        empty_chair: None,
                        user_feedback: None,
                        usage: synth_usage,
            }).await;
                }

                if let Err(e) = store.archive_synthesis(session_id, &content).await {
                    tracing::error!("Failed to archive synthesis: {}", e);
                }
                let msg = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: session_id.to_string(),
                    role: foundation::MessageRole::Synthesis,
                    soul_name: None,
                    content: content.clone(),
                    seq: 0,
                    created_at: chrono::Utc::now(),
                };
                if let Err(e) = store.append_message(&msg).await {
                    tracing::error!("Failed to store synthesis message: {}", e);
                }

                // ── 提取推荐补充魂 ──
                let recommendations = extract_recommended_souls(&content);
                if !recommendations.is_empty() {
                    let payload = serde_json::json!({
                        "recommendations": recommendations,
            });
                    let _ = system_tx.try_send(WsEvent {
                        event_type: WsEventType::SoulRecommendations,
                        payload: payload.to_string(),
                        reasoning_content: None,
                        soul_name: None,
                        seq: 0,
            }).ok();
                }

                let card_content = content;
                let card_prompt = Prompt {
                    messages: vec![PromptMessage {
                        role: "user".into(),
                        content: format!("从以下辩证综合报告中提取最核心的 ≤500 字的卡片：\n\n{}", truncate_char_boundary(&card_content, 3000)),
                        reasoning_content: None,
                        ..Default::default()
            }],
                };
                let card_config = CallConfig { temperature: 0.3, max_tokens: 256, stream: false, ..Default::default() };
                let card_req = LLMRequest { provider: card_provider, prompt: card_prompt, config: card_config };

                // ── Knowledge card + Annotation pass: parallel LLM calls ──
                let annotation_prompt = prompt_builder.build_annotation_prompt(task, &synthesis_outputs);
                let annotation_req = LLMRequest {
                    provider: synthesis_provider.clone(),
                    prompt: annotation_prompt,
                    config: CallConfig { temperature: 0.4, max_tokens: 4096, stream: false, ..Default::default() },
                };

                let card_fut = async {
                    if let Ok(mut card_rx) = gateway.call(&card_req) {
                        let mut card = String::new();
                        let mut card_usage = foundation::UsageStats::default();
                        while let Some(r) = card_rx.recv().await {
                            if let Ok(c) = r {
                                if let Some(u) = c.usage { card_usage = u; }
                                card.push_str(&c.content);
            }
                }
                        if !card.is_empty() {
                            let card_entity = KnowledgeCard {
                                id: uuid::Uuid::new_v4().to_string(),
                                title: task.to_string(),
                                content: card,
                                source_soul: None,
                                source_session: Some(session_id.to_string()),
                                tags: souls.to_vec(),
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
            };
                            let _ = store.insert_knowledge_card(&card_entity).await;
                }
                        if card_usage.total_tokens > 0 {
                            let _ = store.record_call(&foundation::CallRecord {
                                id: uuid::Uuid::new_v4().to_string(),
                                session_id: session_id.to_string(),
                                soul_name: "综合官".to_string(),
                                mode: foundation::PossessionMode::Conference,
                                task_summary: task.to_string(),
                                effectiveness: foundation::Effectiveness::Effective,
                                notes: "[knowledge_card]".to_string(),
                                created_at: chrono::Utc::now(),
                                self_negation: None,
                                empty_chair: None,
                                user_feedback: None,
                                usage: card_usage,
            }).await;
                }
            }
                };

                let ann_fut = async {
                    if let Ok(mut ann_rx) = gateway.call(&annotation_req) {
                    let mut raw = String::new();
                    let mut ann_usage = foundation::UsageStats::default();
                    while let Some(r) = ann_rx.recv().await {
                        if let Ok(c) = r {
                            if let Some(u) = c.usage { ann_usage = u; }
                            raw.push_str(&c.content);
                }
            }
                    if ann_usage.total_tokens > 0 {
                        let _ = store.record_call(&foundation::CallRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: session_id.to_string(),
                            soul_name: "批注官".to_string(),
                            mode: foundation::PossessionMode::Conference,
                            task_summary: task.to_string(),
                            effectiveness: foundation::Effectiveness::Effective,
                            notes: "[annotation]".to_string(),
                            created_at: chrono::Utc::now(),
                            self_negation: None,
                            empty_chair: None,
                            user_feedback: None,
                            usage: ann_usage,
                }).await;
            }
                    let trimmed = raw.trim();
                    let json_str = trimmed
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();
                    match serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                        Ok(items) => {
                            let now = chrono::Utc::now();
                            let annotations: Vec<foundation::Annotation> = items.iter().filter_map(|v| {
                                Some(foundation::Annotation {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    session_id: session_id.to_string(),
                                    source_soul: v.get("source_soul")?.as_str()?.to_string(),
                                    target_soul: v.get("target_soul")?.as_str()?.to_string(),
                                    target_excerpt: v.get("target_excerpt")?.as_str()?.to_string(),
                                    comment: v.get("comment")?.as_str()?.to_string(),
                                    kind: v.get("kind").and_then(|k| k.as_str()).unwrap_or("nuance").to_string(),
                                    created_at: now,
                })
            }).collect();
                            if !annotations.is_empty() {
                                match store.insert_annotations(&annotations).await {
                                    Ok(_) => {
                                        let payload = serde_json::to_string(&annotations)
                                            .unwrap_or_else(|_| "[]".to_string());
                                        let _ = system_tx.try_send(WsEvent {
                                            event_type: WsEventType::AnnotationsReady,
                                            payload,
                                            reasoning_content: None,
                                            soul_name: None,
                                            seq: 0,
                        }).ok();
                                        tracing::info!(
                                            "Persisted {} marginalia annotations for session {}",
                                            annotations.len(),
                                            session_id
                                );
                    }
                                    Err(e) => {
                                        tracing::error!("Failed to persist annotations: {}", e);
                    }
                }
            }
                }
                        Err(e) => {
                    tracing::warn!("Failed to parse annotation JSON: {} (raw len={})", e, raw.len());
                }
            }
            }
                };

                tokio::join!(card_fut, ann_fut);
            }
        }

        // ── 24小时可检验项 ──
        let verify_msg = Message {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: foundation::MessageRole::System,
            soul_name: Some("实践检验".into()),
            content: "## ⏳ 24小时可检验项\n\n综合以上分析，请设定一个**在未来24小时内可以实际检验的具体行动**：\n\n- 你准备做什么来验证（或挑战）这次分析的结论？\n- 你预计什么信号表示\"分析有效\"？什么信号表示\"分析需要修正\"？\n- 检验后，请在下次附体时通过实践开口带回现场数据。\n\n**如果24小时内不检验——本次分析的结论标记为\"待验证\"而非\"已确认\"。**".into(),
            seq: 2,
            created_at: chrono::Utc::now(),
        };
        let _ = store.append_message(&verify_msg).await;
        let _ = system_tx.try_send(WsEvent {
            event_type: WsEventType::SystemMessage,
            payload: "⏳ 请设定一个24小时内可检验的具体行动，验证本次分析的结论。".into(),
            reasoning_content: None,
            soul_name: None,
            seq: 0,
        }).ok();
    }

    Ok(outputs)
}

async fn run_soul_with_tools(
    gw: &GatewayRegistry,
    provider: &foundation::Provider,
    prompt: &foundation::Prompt,
    config: &foundation::CallConfig,
    session_id: &str,
    soul_name: &str,
    ws: &WsSessionManager,
    tool_registry: &crate::tools::ToolRegistry,
    detector: CrossDetector,
) -> SoulOutput {
    let max_rounds = if let Some(tools) = &config.tools {
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        crate::tools::max_tool_rounds_for_tools(&names)
    } else {
        crate::tools::max_tool_rounds()
    };
    let mut history: Vec<foundation::PromptMessage> = prompt.messages.clone();
    let name = soul_name.to_string();
    let mut used_providers: Vec<foundation::Provider> = vec![provider.clone()];
    let mut current_provider = provider.clone();
    // 追踪最后一轮 assistant 实际产出的文本，用于循环超限时降级返回（而非丢弃）
    let mut last_assistant_content = String::new();

    for round in 0..max_rounds {
        let req = foundation::LLMRequest {
            provider: current_provider.clone(),
            prompt: foundation::Prompt { messages: history.clone() },
            config: config.clone(),
        };

        let rx = match gw.call(&req) {
            Ok(rx) => rx,
            Err(e) => {
                let error_msg = e.to_string();
                if !error_msg.contains("400") {
                    gw.mark_provider_unhealthy(&current_provider, error_msg.clone());
                }

                if let Some(next_provider) = gw.try_next_provider(&current_provider) {
                    if !used_providers.contains(&next_provider) {
                        tracing::warn!(
                            "Soul '{}' provider {:?} failed, trying fallback {:?}: {}",
                            name, current_provider, next_provider, error_msg
                        );
                        used_providers.push(next_provider);
                        current_provider = next_provider;
                        history = prompt.messages.clone();
                        continue;
            }
                }

                ws.broadcast_soul(
                    session_id,
                    &name,
                    &WsEvent {
                        event_type: WsEventType::SoulError,
                        payload: error_msg.clone(),
                        reasoning_content: None,
                        soul_name: Some(name.clone()),
                        seq: 0,
            },
                );
                return SoulOutput::error(name, error_msg);
            }
        };

        let output = stream_single_soul_with_detection_with_provider(
            rx,
            session_id,
            &name,
            ws,
            detector.clone(),
        ).await;

        if output.error.is_some() {
            let err_msg = output.error.clone().unwrap_or_default();
            // 400 错误是 API 参数问题不是服务宕机，不标记 unhealthy
            if !err_msg.contains("400") {
                gw.mark_provider_unhealthy(&current_provider, err_msg.clone());
            }
            if let Some(next_provider) = gw.try_next_provider(&current_provider) {
                if !used_providers.contains(&next_provider) {
                    tracing::warn!(
                        "Soul '{}' provider {:?} stream failed, trying fallback {:?}",
                        name, current_provider, next_provider
                    );
                    used_providers.push(next_provider);
                    current_provider = next_provider;
                    // 重置 history 为原始 prompt，防止前一个 provider 残留的 tool_calls 污染
                    history = prompt.messages.clone();
                    continue;
                }
            }
            return output;
        }

        if output.tool_calls.is_empty() {
            gw.mark_provider_healthy(&current_provider);
            return output;
        }

        // 工具调用成功执行 → 标记 provider 健康
        gw.mark_provider_healthy(&current_provider);

        // 记录本轮 assistant 实际产出的文本（用于循环超限时降级返回）
        if !output.content.trim().is_empty() {
            last_assistant_content = output.content.clone();
        }

        // 先追加 assistant 消息（含所有 tool_calls 和原始文本），再逐个追加 tool 结果
        history.push(foundation::PromptMessage {
            role: "assistant".to_string(),
            content: output.content.clone(),
            reasoning_content: Some(String::new()),
            tool_calls: Some(output.tool_calls.clone()),
            tool_call_id: None,
        });

        for tc in &output.tool_calls {
                    let payload = crate::ToolCallPayload {
                        tool_call_id: tc.id.clone(),
                        tool_name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                        soul_name: name.clone(),
            };
                    let json = serde_json::to_string(&payload).unwrap_or_default();
                    ws.broadcast_soul(session_id, &name, &WsEvent {
                        event_type: WsEventType::ToolCallStarted,
                        payload: json,
                        reasoning_content: None,
                        soul_name: Some(name.clone()),
                        seq: 0,
            });

                    match tool_registry.execute(tc).await {
                        Ok(result) => {
                            let result_payload = crate::ToolResultPayload {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.function.name.clone(),
                                result: result.clone(),
                                soul_name: name.clone(),
            };
                            let result_json = serde_json::to_string(&result_payload).unwrap_or_default();
                            ws.broadcast_soul(session_id, &name, &WsEvent {
                                event_type: WsEventType::ToolResult,
                                payload: result_json,
                                reasoning_content: None,
                                soul_name: Some(name.clone()),
                                seq: 0,
            });

                            history.push(foundation::PromptMessage {
                                role: "tool".to_string(),
                                content: result,
                                reasoning_content: None,
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
            });
                }
                        Err(e) => {
                            let error_msg = format!("Tool {} failed: {}", tc.function.name, e);
                            ws.broadcast_soul(
                                session_id,
                                &name,
                                &WsEvent {
                                    event_type: WsEventType::SoulError,
                                    payload: error_msg.clone(),
                                    reasoning_content: None,
                                    soul_name: Some(name.clone()),
                                    seq: 0,
                },
                    );
                            return SoulOutput::error(name, error_msg);
                }
            }
                }

        // 最后一轮工具调用结束后，注入收尾引导：要求模型停止调工具、直接输出最终陈述
        if round + 1 == max_rounds - 1 {
            history.push(foundation::PromptMessage {
                role: "user".to_string(),
                content: "你已经收集了足够的信息。现在请停止调用任何工具，直接输出你的最终陈述。不要再使用 search_labor_law / calculate_severance / generate_evidence_checklist / WebSearch，把已掌握的事实和法条整理成完整的发言。".to_string(),
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
            }

    // 循环超限：降级返回最后一轮 assistant 实际产出的文本，而非完全丢弃
    if !last_assistant_content.trim().is_empty() {
        tracing::warn!(
            "Soul '{}' exceeded {} tool rounds, returning last assistant content ({} chars) as degraded output",
            name, max_rounds, last_assistant_content.len()
        );
        let warn_msg = format!("（已达到工具调用上限 {} 轮，以下为已生成的陈述）", max_rounds);
        return SoulOutput {
            soul_name: name,
            content: format!("{}\n\n{}", last_assistant_content, warn_msg),
            usage: foundation::UsageStats::default(),
            error: None,
            tool_calls: Vec::new(),
        };
    }

    let error_msg = format!("Tool call loop exceeded {} rounds", max_rounds);
    ws.broadcast_soul(
        session_id,
        &name,
        &WsEvent {
            event_type: WsEventType::SoulError,
            payload: error_msg.clone(),
            reasoning_content: None,
            soul_name: Some(name.clone()),
            seq: 0,
        },
    );
    SoulOutput {
        soul_name: name,
        content: String::new(),
        usage: foundation::UsageStats::default(),
        error: Some(error_msg),
        tool_calls: Vec::new(),
    }
}

/// 将干预决策转为注入魂对话的 user 消息
async fn stream_single_soul_with_detection_with_provider(
    mut rx: mpsc::Receiver<foundation::Result<foundation::Chunk>>,
    session_id: &str,
    soul_name: &str,
    ws: &WsSessionManager,
    detector: CrossDetector,
) -> SoulOutput {
    let mut content = String::new();
    let mut usage = foundation::UsageStats::default();
    let mut seq: u32 = 0;
    let name = soul_name.to_string();
    let mut tool_calls: Vec<foundation::ToolCall> = Vec::new();
    let mut truncated = false;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(chunk) => {
                if let Some(u) = chunk.usage {
                    usage = u;
                }
                if !chunk.tool_calls.is_empty() {
                    tool_calls.extend(chunk.tool_calls);
                }
                if !chunk.content.is_empty() {
                    content.push_str(&chunk.content);

                    ws.broadcast_soul(
                        session_id,
                        &name,
                        &WsEvent {
                            event_type: WsEventType::SoulChunk,
                            payload: chunk.content.clone(),
                            reasoning_content: chunk.reasoning_content.clone(),
                            soul_name: Some(name.clone()),
                            seq,
                },
                    );

                    // 直接调用检测器，不走广播通道
                    detector.add_token(&name, &chunk.content);

                    seq += 1;
                } else if let Some(ref reasoning) = chunk.reasoning_content {
                    if !reasoning.is_empty() && seq == 0 {
                        ws.broadcast_soul(
                            session_id,
                            &name,
                            &WsEvent {
                                event_type: WsEventType::SoulChunk,
                                payload: String::new(),
                                reasoning_content: Some(reasoning.clone()),
                                soul_name: Some(name.clone()),
                                seq,
            },
                        );
                        seq += 1;
            }
                }
                if chunk.finish_reason.as_deref() == Some("length") {
                    truncated = true;
                    tracing::warn!("Soul '{}' output truncated by max_tokens limit", name);
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                ws.broadcast_soul(
                    session_id,
                    &name,
                    &WsEvent {
                        event_type: WsEventType::SoulError,
                        payload: error_msg.clone(),
                        reasoning_content: None,
                        soul_name: Some(name.clone()),
                        seq,
            },
                );
                return SoulOutput::error(name, error_msg);
            }
        }
    }

    ws.broadcast_soul(
        session_id,
        &name,
        &WsEvent {
            event_type: WsEventType::SoulDone,
            payload: String::new(),
            reasoning_content: None,
            soul_name: Some(name.clone()),
            seq,
        },
    );

    if truncated {
        content.push_str("\n\n> ⚠️ [系统提示] 输出因长度限制被截断。如需完整分析，可尝试简化任务或分拆问题。");
    }

    let output = SoulOutput { soul_name: name.clone(), content, usage, error: None, tool_calls };

    output
}

/// 从综合报告中提取"七、推荐补充魂"部分的建议魂名
fn extract_recommended_souls(synthesis: &str) -> Vec<serde_json::Value> {
    let section_start = synthesis.find("## 七、推荐补充魂");
    let section = match section_start {
        Some(start) => &synthesis[start..],
        None => return vec![],
    };

    // 截取到下一个一级标题或文件末尾
    let section_end = section[1..].find("\n## ").map(|i| i + 1).unwrap_or(section.len());
    let section_text = &section[..section_end];

    // 如果明确写"无需补充"，返回空
    if section_text.contains("无需补充") {
        return vec![];
    }

    let mut results = Vec::new();
    // 匹配 Markdown 列表项：- **魂名**：推荐理由...
    for line in section_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("-") || trimmed.starts_with("*") {
            if let Some(name_start) = trimmed.find("**") {
                let after_first = &trimmed[name_start + 2..];
                if let Some(name_end) = after_first.find("**") {
                    let name = after_first[..name_end].trim();
                    if !name.is_empty() && name.len() < 50 {
                        // 提取推荐理由（**魂名**：后面的内容）
                        let rationale = after_first[name_end + 2..]
                            .trim_start_matches(|c| c == '：' || c == ':' || c == ' ')
                            .trim();
                        results.push(serde_json::json!({
                            "name": name,
                            "rationale": if rationale.is_empty() { "综合官推荐补充" } else { rationale },
                }));
            }
                }
            }
        }
    }

    results
}
