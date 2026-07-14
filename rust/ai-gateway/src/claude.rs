use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use foundation::{CallConfig, Chunk, FoundationError, Prompt, Provider, Result, UsageStats};
use reqwest::Client;
use tokio::sync::mpsc;

use crate::Gateway;

fn serialize_claude_message(m: &foundation::PromptMessage) -> serde_json::Value {
    match m.role.as_str() {
        "assistant" if m.tool_calls.is_some() => {
            let blocks: Vec<serde_json::Value> = m
                .tool_calls
                .as_ref()
                .unwrap()
                .iter()
                .map(|tc| {
                    let input: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| {
                            tracing::warn!(
                                "Tool call arguments not valid JSON for '{}', sending as-is: {}",
                                tc.function.name,
                                &tc.function.arguments[..tc.function.arguments.len().min(200)]
                            );
                            serde_json::json!({ "_raw": tc.function.arguments })
                        });
                    serde_json::json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.function.name,
                        "input": input
                    })
                })
                .collect();
            serde_json::json!({"role": "assistant", "content": blocks})
        }
        "tool" => {
            serde_json::json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": m.tool_call_id,
                    "content": m.content
                }]
            })
        }
        _ => {
            serde_json::json!({"role": m.role, "content": m.content})
        }
    }
}

pub struct ClaudeClient {
    api_key: Option<String>,
    pub model: String,
    client: Client,
    dynamic_base_url: Option<Arc<RwLock<String>>>,
    dynamic_api_key: Option<Arc<RwLock<Option<String>>>>,
    dynamic_model: Option<Arc<RwLock<String>>>,
}

impl ClaudeClient {
    pub fn new() -> Self {
        let api_key = crate::load_api_key("ANTHROPIC_API_KEY", "anthropic");
        let model =
            std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-4-6".into());
        ClaudeClient {
            api_key,
            model,
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("Failed to build reqwest Client"),
            dynamic_base_url: Some(Arc::new(RwLock::new(String::new()))),
            dynamic_api_key: Some(Arc::new(RwLock::new(None))),
            dynamic_model: Some(Arc::new(RwLock::new(String::new()))),
        }
    }
}

impl ClaudeClient {
    fn effective_base_url(&self) -> String {
        if let Some(ref lock) = self.dynamic_base_url {
            let url = lock.read().clone();
            if !url.is_empty() {
                return url;
            }
        }
        crate::relay_base_url("https://api.anthropic.com/v1")
    }

    fn effective_api_key(&self) -> Option<String> {
        if let Some(ref lock) = self.dynamic_api_key {
            let key_guard = lock.read();
            if let Some(ref key) = *key_guard {
                if !key.is_empty() {
                    return Some(key.clone());
                }
            }
        }
        self.api_key.clone()
    }

    fn effective_model(&self) -> String {
        if let Some(ref lock) = self.dynamic_model {
            let model = lock.read().clone();
            if !model.is_empty() {
                return model;
            }
        }
        self.model.clone()
    }

    pub fn set_dynamic_base_url(&self, url: String) {
        if let Some(ref lock) = self.dynamic_base_url {
            *lock.write() = url;
        }
    }

    pub fn set_dynamic_api_key(&self, key: Option<String>) {
        if let Some(ref lock) = self.dynamic_api_key {
            *lock.write() = key;
        }
    }

    pub fn set_dynamic_model(&self, model: String) {
        if let Some(ref lock) = self.dynamic_model {
            *lock.write() = model;
        }
    }
}

impl Gateway for ClaudeClient {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn is_available(&self) -> bool {
        self.effective_api_key().is_some()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn call(
        &self,
        prompt: &Prompt,
        config: &CallConfig,
    ) -> mpsc::Receiver<Result<Chunk>> {
        let (tx, rx) = mpsc::channel(256);
        let api_key = match self.effective_api_key() {
            Some(k) => k,
            None => {
                let _ = tx.try_send(Err(FoundationError::Validation(
                    "Claude API key not configured".into(),
                )));
                return rx;
            }
        };
        let model = self.effective_model();
        let client = self.client.clone();
        let config = config.clone();

        let system = prompt
            .messages
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");

        let messages: Vec<serde_json::Value> = prompt
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(serialize_claude_message)
            .collect();

        let mut body = if system.is_empty() {
            serde_json::json!({
                "model": model,
                "max_tokens": config.max_tokens,
                "temperature": config.temperature,
                "messages": messages,
                "stream": false,
            })
        } else {
            serde_json::json!({
                "model": model,
                "max_tokens": config.max_tokens,
                "temperature": config.temperature,
                "system": system,
                "messages": messages,
                "stream": false,
            })
        };

        if let Some(tools) = &config.tools {
            let claude_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(claude_tools);
        }

        let effective_url = self.effective_base_url();
        tokio::spawn(async move {

            let result = client
                .post(format!("{}/messages", effective_url))
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                // Claude Code 伪装头
                .header("user-agent", "claude-cli/1.0.83 (external, cli)")
                .header("x-app", "cli")
                .json(&body)
                .send()
                .await;

            let response = match result {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Err(FoundationError::Io(std::io::Error::other(e.to_string())))).await;
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                // 诊断：打印所有消息的角色和 tool 状态
                let msg_roles: Vec<String> = messages.iter().map(|m| {
                    let role = m["role"].as_str().unwrap_or("?");
                    let blocks = m["content"].as_array();
                    let tool_uses: Vec<String> = blocks.map_or(vec![], |arr| 
                        arr.iter().filter(|b| b["type"] == "tool_use")
                           .map(|b| b["id"].as_str().unwrap_or("?").to_string())
                           .collect()
                    );
                    let tool_results: Vec<String> = blocks.map_or(vec![], |arr|
                        arr.iter().filter(|b| b["type"] == "tool_result")
                           .map(|b| b["tool_use_id"].as_str().unwrap_or("?").to_string())
                           .collect()
                    );
                    format!("{}[uses:{:?} results:{:?}]", role, tool_uses, tool_results)
                }).collect();
                tracing::error!("Claude {} error: {} | msgs={} roles={:?}", 
                    status, &body[..body.len().min(300)], messages.len(), msg_roles);
                let _ = tx.send(Err(FoundationError::Validation(format!(
                    "Claude API error {}: {}",
                    status, body
                )))).await;
                return;
            }

            #[allow(unused_imports)]
            use futures::StreamExt;
            // 非流式：读取完整响应，解析 JSON body
            let body_text = response.text().await.unwrap_or_default();
            tracing::info!("Claude response: {} bytes", body_text.len());
            let resp: serde_json::Value = match serde_json::from_str(&body_text) {
                Ok(v) => v,
                Err(e) => { let _ = tx.send(Err(FoundationError::Validation(format!("Claude parse error: {}", e)))).await; return; }
            };
            let mut full_text = String::new();
            let mut tool_calls = Vec::new();
            if let Some(blocks) = resp["content"].as_array() {
                for b in blocks {
                    match b["type"].as_str() {
                        Some("text") => {
                            if let Some(t) = b["text"].as_str() { full_text.push_str(t); }
                        }
                        Some("tool_use") => {
                            let id = b["id"].as_str().unwrap_or("").to_string();
                            let name = b["name"].as_str().unwrap_or("").to_string();
                            let args = b["input"].to_string();
                            tool_calls.push(foundation::ToolCall {
                                id,
                                r#type: "function".to_string(),
                                function: foundation::ToolCallFunction { name, arguments: args },
                            });
                        }
                        _ => {}
                    }
                }
            }
            let it = resp["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
            let ot = resp["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
            if !full_text.is_empty() || !tool_calls.is_empty() {
                let _ = tx.send(Ok(Chunk { content: full_text, reasoning_content: None, finish_reason: None, index: 0, usage: None, tool_calls })).await;
            }
            let _ = tx.send(Ok(Chunk { content: String::new(), reasoning_content: None, finish_reason: Some(resp["stop_reason"].as_str().unwrap_or("end_turn").to_string()), index: 1, usage: Some(UsageStats { prompt_tokens: it, completion_tokens: ot, total_tokens: it + ot }), tool_calls: vec![] })).await;
        });
        rx
    }
}
