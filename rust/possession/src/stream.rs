use ai_gateway::GatewayRegistry;
use foundation::{Chunk, LLMRequest, Prompt, PromptMessage, Provider, ProviderInfo, Result, UsageStats};
use tokio::sync::mpsc;

use crate::{SoulOutput, ToolCallPayload, ToolResultPayload, WsEvent, WsEventType, WsSessionManager};
use crate::tools::ToolRegistry;

pub async fn stream_single_soul_with_provider(
    rx: mpsc::Receiver<Result<Chunk>>,
    session_id: &str,
    soul_name: &str,
    ws: &WsSessionManager,
    _gateway: &GatewayRegistry,
    _provider: &Provider,
) -> SoulOutput {
    let mut content = String::new();
    let mut usage = UsageStats::default();
    let mut seq: u32 = 0;
    let name = soul_name.to_string();
    let mut tool_calls: Vec<foundation::ToolCall> = Vec::new();
    let mut truncated = false;

    let mut rx = rx;

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
                }
                if !chunk.content.is_empty() || chunk.reasoning_content.is_some() {
                    ws.broadcast_soul(
                        session_id,
                        &name,
                        &WsEvent {
                            event_type: WsEventType::SoulChunk,
                            payload: chunk.content,
                            reasoning_content: chunk.reasoning_content,
                            soul_name: Some(name.clone()),
                            seq,
                        },
                    );
                    seq += 1;
                }
                if chunk.finish_reason.as_deref() == Some("length") {
                    truncated = true;
                    tracing::warn!("Soul '{}' output truncated by max_tokens limit", name);
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                // 瞬断不封杀 — 让 conference 自行 fallback
tracing::warn!("Soul stream error for {}: {}", soul_name, error_msg);
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

    SoulOutput { soul_name: name, content, usage, error: None, tool_calls }
}

/// Stream LLM chunks to WebSocket, aggregating content and usage stats.
pub async fn stream_single_soul(
    mut rx: mpsc::Receiver<Result<Chunk>>,
    session_id: &str,
    soul_name: &str,
    ws: &WsSessionManager,
) -> SoulOutput {
    let mut content = String::new();
    let mut usage = UsageStats::default();
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
                // 修复：content 和 reasoning_content 同时存在时也要累积 content
                if !chunk.content.is_empty() {
                    content.push_str(&chunk.content);
                }
                if !chunk.content.is_empty() || chunk.reasoning_content.is_some() {
                    ws.broadcast_soul(
                        session_id,
                        &name,
                        &WsEvent {
                            event_type: WsEventType::SoulChunk,
                            payload: chunk.content,
                            reasoning_content: chunk.reasoning_content,
                            soul_name: Some(name.clone()),
                            seq,
                        },
                    );
                    seq += 1;
                }
                // 检测 max_tokens 截断
                if chunk.finish_reason.as_deref() == Some("length") {
                    truncated = true;
                    tracing::warn!("Soul '{}' output truncated by max_tokens limit", name);
                }
            }
            Err(e) => {
                ws.broadcast_soul(
                    session_id,
                    &name,
                    &WsEvent {
                        event_type: WsEventType::SoulError,
                        payload: e.to_string(),
                        reasoning_content: None,
                        soul_name: Some(name.clone()),
                        seq,
                    },
                );
                return SoulOutput::error(name, e.to_string());
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

    SoulOutput { soul_name: name, content, usage, error: None, tool_calls }
}

/// Pick provider info respecting preferred_provider, falling back to first available.
pub fn pick_provider_info(gateway: &GatewayRegistry) -> ProviderInfo {
    gateway.pick_provider_info()
}

/// Stream synthesis to WebSocket, returning aggregated content and usage.
pub async fn stream_synthesis(
    mut rx: mpsc::Receiver<Result<Chunk>>,
    session_id: &str,
    ws: &WsSessionManager,
    system_tx: &mpsc::Sender<WsEvent>,
) -> Result<(String, UsageStats)> {
    tracing::info!("Starting stream_synthesis for session: {}", session_id);
    let mut content = String::new();
    let mut usage = UsageStats::default();
    let mut seq: u32 = 0;
    let mut chunk_count = 0;
    let mut truncated = false;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(chunk) => {
                if let Some(u) = chunk.usage {
                    usage = u;
                }
                if !chunk.content.is_empty() {
                    content.push_str(&chunk.content);
                }
                if !chunk.content.is_empty() || chunk.reasoning_content.is_some() {
                    chunk_count += 1;
                    let event = WsEvent {
                        event_type: WsEventType::SynthesisChunk,
                        payload: chunk.content,
                        reasoning_content: chunk.reasoning_content,
                        soul_name: None,
                        seq,
                    };
                    ws.broadcast_system(session_id, &event);
                    let _ = system_tx.try_send(event).ok();
                    seq += 1;
                }
                // 检测 max_tokens 截断
                if chunk.finish_reason.as_deref() == Some("length") {
                    truncated = true;
                    tracing::warn!("Synthesis output truncated by max_tokens limit");
                }
            }
            Err(e) => {
                tracing::error!("Error in synthesis stream: {}", e);
                return Err(e);
            }
        }
    }

    tracing::info!("Stream complete, {} chunks, total content length: {}", chunk_count, content.len());
    let done_event = WsEvent {
        event_type: WsEventType::SynthesisDone,
        payload: String::new(),
        reasoning_content: None,
        soul_name: None,
        seq,
    };
    ws.broadcast_system(session_id, &done_event);
    let _ = system_tx.try_send(done_event).ok();

    if truncated {
        content.push_str("\n\n> ⚠️ [系统提示] 综合报告因长度限制被截断。");
    }

    Ok((content, usage))
}

pub async fn run_tool_loop(
    gateway: &GatewayRegistry,
    provider: foundation::Provider,
    initial_prompt: &Prompt,
    config: &foundation::CallConfig,
    session_id: &str,
    soul_name: &str,
    ws: &WsSessionManager,
    tool_registry: &ToolRegistry,
    max_rounds: usize,
) -> SoulOutput {
    let mut history: Vec<PromptMessage> = initial_prompt.messages.clone();
    let name = soul_name.to_string();
    let mut used_providers: Vec<foundation::Provider> = vec![provider.clone()];
    let mut current_provider = provider;

    for _round in 0..max_rounds {
        let prompt = Prompt { messages: history.clone() };
        let req = LLMRequest {
            provider: current_provider.clone(),
            prompt,
            config: config.clone(),
        };

        let rx = match gateway.call(&req) {
            Ok(rx) => rx,
            Err(e) => {
                let error_msg = e.to_string();
                tracing::warn!("Tool loop: provider {:?} failed: {} — trying fallback", current_provider, error_msg);

                if let Some(next_provider) = gateway.try_next_provider(&current_provider) {
                    if !used_providers.contains(&next_provider) {
                        tracing::warn!(
                            "Soul '{}' provider {:?} failed, trying fallback {:?}: {}",
                            name, current_provider, next_provider, error_msg
                        );
                        used_providers.push(next_provider);
                        current_provider = next_provider;
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

        let output = stream_single_soul_with_provider(rx, session_id, soul_name, ws, gateway, &current_provider).await;

        if output.error.is_some() {
            if let Some(next_provider) = gateway.try_next_provider(&current_provider) {
                if !used_providers.contains(&next_provider) {
                    tracing::warn!(
                        "Soul '{}' provider {:?} stream failed, trying fallback {:?}",
                        name, current_provider, next_provider
                    );
                    used_providers.push(next_provider);
                    current_provider = next_provider;
                    continue;
                }
            }
            return output;
        }

        if output.tool_calls.is_empty() {
            return output;
        }

        // 先推 assistant 消息（含所有 tool_calls），再逐个推 tool_result
        history.push(PromptMessage {
            role: "assistant".to_string(),
            content: String::new(),
            reasoning_content: Some(String::new()),
            tool_calls: Some(output.tool_calls.clone()),
            tool_call_id: None,
        });

        for tc in &output.tool_calls {
            let payload = ToolCallPayload {
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
                    let result_payload = ToolResultPayload {
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

                    history.push(PromptMessage {
                        role: "assistant".to_string(),
                        content: String::new(),
                        reasoning_content: Some(String::new()),
                        tool_calls: Some(output.tool_calls.clone()),
                        tool_call_id: None,
                    });
                    history.push(PromptMessage {
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
        usage: UsageStats::default(),
        error: Some(error_msg),
        tool_calls: Vec::new(),
    }
}
