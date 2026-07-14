use std::sync::{Arc, PoisonError, RwLock};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::{map_api_error, ApiError};
use crate::state::AppState;

/// 安全读取全局配置锁：中毒时取出内部数据继续运行，而不是让整个 HTTP 进程 panic。
/// （std::sync::RwLock 一旦持锁者在 panic 中死亡就会中毒，后续 .unwrap() 会再次 panic。）
fn read_cfg<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// 安全写入全局配置锁：同上，中毒时仍可恢复写入。
fn write_cfg<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

const DEFAULT_MODEL_FILE: &str = "data/default-model.json";
const LMSTUDIO_CONFIG_FILE: &str = "data/lmstudio.json";
const OPENAI_CONFIG_FILE: &str = "data/openai.json";
const CLAUDE_CONFIG_FILE: &str = "data/claude.json";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/model", post(set_default_model).get(get_default_model))
        .route("/provider", post(set_provider).get(get_provider))
        .route("/provider/test", post(test_provider))
        .route("/providers", get(list_providers))
        .route("/lmstudio-url", post(set_lmstudio_url).get(get_lmstudio_url))
        .route("/lmstudio-key", post(set_lmstudio_key).get(get_lmstudio_key))
        .route("/lmstudio-model", post(set_lmstudio_model).get(get_lmstudio_model))
        .route("/openai-url", post(set_openai_url).get(get_openai_url))
        .route("/openai-key", post(set_openai_key).get(get_openai_key))
        .route("/openai-model", post(set_openai_model).get(get_openai_model))
        .route("/openai/test", post(test_openai))
        .route("/claude-url", post(set_claude_url).get(get_claude_url))
        .route("/claude-key", post(set_claude_key).get(get_claude_key))
        .route("/claude-model", post(set_claude_model).get(get_claude_model))
        .route("/claude/test", post(test_claude))
        .route("/balance", get(check_balance))
        .route("/relay", post(set_relay).get(get_relay))
        .route("/relay/test", post(test_relay))
        .route("/domain", get(get_domain_info).post(set_domain))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model: String,
    pub reasoning: String,
}

fn load_model_config() -> ModelConfig {
    if let Ok(content) = std::fs::read_to_string(DEFAULT_MODEL_FILE) {
        if let Ok(config) = serde_json::from_str::<ModelConfig>(&content) {
            return config;
        }
    }
    ModelConfig {
        model: std::env::var("AIONUI_DEFAULT_MODEL").unwrap_or_else(|_| "deepseek-v4-pro".to_string()),
        reasoning: std::env::var("AIONUI_DEFAULT_REASONING").unwrap_or_else(|_| "think".to_string()),
    }
}

fn save_model_config(config: &ModelConfig) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        // ensure parent directory exists
        if let Some(parent) = std::path::Path::new(DEFAULT_MODEL_FILE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(DEFAULT_MODEL_FILE, json);
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LmStudioConfig {
    #[serde(default)]
    url: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    model: String,
}

fn load_lmstudio_config() -> LmStudioConfig {
    if let Ok(content) = std::fs::read_to_string(LMSTUDIO_CONFIG_FILE) {
        if let Ok(config) = serde_json::from_str::<LmStudioConfig>(&content) {
            return config;
        }
    }
    LmStudioConfig::default()
}

fn save_lmstudio_config(config: &LmStudioConfig) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Some(parent) = std::path::Path::new(LMSTUDIO_CONFIG_FILE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(LMSTUDIO_CONFIG_FILE, json);
    }
}

lazy_static::lazy_static! {
    static ref DEFAULT_MODEL_CONFIG: RwLock<ModelConfig> = RwLock::new(load_model_config());
}

lazy_static::lazy_static! {
    static ref LMSTUDIO_CONFIG: RwLock<LmStudioConfig> = RwLock::new(load_lmstudio_config());
}

#[derive(Debug, Deserialize)]
struct SetModelRequest {
    model: String,
    #[serde(default = "default_reasoning")]
    reasoning: String,
}

fn default_reasoning() -> String {
    std::env::var("AIONUI_DEFAULT_REASONING").unwrap_or_else(|_| "think".to_string())
}

#[derive(Debug, Serialize)]
struct ModelResponse {
    model: String,
    reasoning: String,
}

async fn get_default_model() -> Json<ModelResponse> {
    let config = read_cfg(&DEFAULT_MODEL_CONFIG);
    Json(ModelResponse {
        model: config.model.clone(),
        reasoning: config.reasoning.clone(),
    })
}

async fn set_default_model(
    Json(body): Json<SetModelRequest>,
) -> Result<Json<ModelResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    let mut config = write_cfg(&DEFAULT_MODEL_CONFIG);
    config.model = body.model;
    config.reasoning = body.reasoning;
    save_model_config(&config);
    Ok(Json(ModelResponse {
        model: config.model.clone(),
        reasoning: config.reasoning.clone(),
    }))
}

#[derive(Debug, Deserialize)]
struct SetProviderRequest {
    provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProviderResponse {
    provider: Option<String>,
}

async fn get_provider(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderResponse> {
    let provider = read_cfg(&state.preferred_provider).map(|p| format!("{:?}", p).to_lowercase());
    Json(ProviderResponse { provider })
}

async fn set_provider(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderRequest>,
) -> Result<Json<ProviderResponse>, (axum::http::StatusCode, Json<ApiError>)> {
    let p = match body.provider.as_deref() {
        Some("claude") | Some("anthropic") => Some(foundation::Provider::Claude),
        Some("openai") => Some(foundation::Provider::OpenAI),
        Some("deepseek") => Some(foundation::Provider::DeepSeek),
        Some("lmstudio") => Some(foundation::Provider::LMStudio),
        Some("") | None => None,
        _ => return Err((axum::http::StatusCode::BAD_REQUEST, Json(ApiError { error: "未知 provider".into() }))),
    };
    {
        let mut pref = write_cfg(&state.preferred_provider);
        *pref = p;
    }
    state.engine.gateway().set_preferred_provider(p);
    let provider = p.map(|p| format!("{:?}", p).to_lowercase());
    Ok(Json(ProviderResponse { provider }))
}

async fn check_balance(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<ApiError>)> {
    let balance = state.engine.gateway().check_deepseek_balance().await.map_err(map_api_error)?;
    Ok(Json(balance))
}

#[derive(Debug, Serialize)]
struct LmStudioUrlResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct SetLmStudioUrlRequest {
    url: String,
}

async fn get_lmstudio_url(
    State(state): State<Arc<AppState>>,
) -> Json<LmStudioUrlResponse> {
    let url = state.engine.gateway().lmstudio_base_url();
    Json(LmStudioUrlResponse { url })
}

async fn set_lmstudio_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetLmStudioUrlRequest>,
) -> Json<LmStudioUrlResponse> {
    state.engine.gateway().set_lmstudio_base_url(body.url.clone());
    let mut cfg = write_cfg(&LMSTUDIO_CONFIG);
    cfg.url = body.url.clone();
    save_lmstudio_config(&cfg);
    Json(LmStudioUrlResponse { url: body.url })
}

#[derive(Debug, Serialize)]
struct LmStudioKeyResponse {
    has_key: bool,
}

#[derive(Debug, Deserialize)]
struct SetLmStudioKeyRequest {
    key: Option<String>,
}

async fn get_lmstudio_key(
    State(state): State<Arc<AppState>>,
) -> Json<LmStudioKeyResponse> {
    let key = state.engine.gateway().lmstudio_api_key();
    Json(LmStudioKeyResponse { has_key: key.is_some() })
}

async fn set_lmstudio_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetLmStudioKeyRequest>,
) -> Json<LmStudioKeyResponse> {
    let key = body.key.filter(|k| !k.is_empty());
    state.engine.gateway().set_lmstudio_api_key(key.clone());
    let has_key = state.engine.gateway().lmstudio_api_key().is_some();
    let mut cfg = write_cfg(&LMSTUDIO_CONFIG);
    cfg.api_key = key.unwrap_or_default();
    save_lmstudio_config(&cfg);
    Json(LmStudioKeyResponse { has_key })
}

#[derive(Debug, Serialize)]
struct LmStudioModelResponse {
    model: String,
}

#[derive(Debug, Deserialize)]
struct SetLmStudioModelRequest {
    model: String,
}

async fn get_lmstudio_model(
    State(state): State<Arc<AppState>>,
) -> Json<LmStudioModelResponse> {
    let model = state.engine.gateway().lmstudio_model();
    Json(LmStudioModelResponse { model })
}

async fn set_lmstudio_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetLmStudioModelRequest>,
) -> Json<LmStudioModelResponse> {
    state.engine.gateway().set_lmstudio_model(body.model.clone());
    let mut cfg = write_cfg(&LMSTUDIO_CONFIG);
    cfg.model = body.model.clone();
    save_lmstudio_config(&cfg);
    Json(LmStudioModelResponse { model: body.model })
}

#[derive(Debug, Serialize)]
struct ProviderStatus {
    id: String,
    name: String,
    model: String,
    available: bool,
    has_key: bool,
    tier: String,
    active: bool,
}

async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ProviderStatus>> {
    let preferred = read_cfg(&state.preferred_provider).clone();
    let gateway = state.engine.gateway();

    let statuses: Vec<ProviderStatus> = gateway.list_providers().into_iter().map(|info| {
        let id = format!("{:?}", info.provider).to_lowercase();
        let tier = format!("{:?}", info.tier);
        let has_key = match info.provider {
            foundation::Provider::LMStudio => true,
            _ => info.available,
        };
        ProviderStatus {
            active: preferred == Some(info.provider),
            id,
            name: match info.provider {
                foundation::Provider::Claude => "Claude".into(),
                foundation::Provider::OpenAI => "OpenAI".into(),
                foundation::Provider::DeepSeek => "DeepSeek".into(),
                foundation::Provider::LMStudio => "LM Studio".into(),
            },
            model: info.model,
            available: info.available,
            has_key,
            tier,
        }
    }).collect();

    Json(statuses)
}

#[derive(Debug, Deserialize)]
struct TestProviderRequest {
    provider: String,
    #[serde(default)]
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestProviderResponse {
    ok: bool,
    message: String,
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
}

async fn test_provider(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TestProviderRequest>,
) -> Json<TestProviderResponse> {
    let provider = match body.provider.as_str() {
        "claude" | "anthropic" => foundation::Provider::Claude,
        "openai" => foundation::Provider::OpenAI,
        "deepseek" => foundation::Provider::DeepSeek,
        "lmstudio" => foundation::Provider::LMStudio,
        _ => return Json(TestProviderResponse { ok: false, message: "未知 provider".into(), latency_ms: None, model: None }),
    };

    // 测试 LM Studio 时，临时设置 API key（如果有提供）
    let _temp_key_guard = if provider == foundation::Provider::LMStudio {
        if let Some(ref key) = body.api_key {
            if !key.is_empty() {
                state.engine.gateway().set_lmstudio_api_key(Some(key.clone()));
            }
        }
        Some(())
    } else {
        None
    };

    // LM Studio：主动探测当前加载的模型名
    let detected_model: Option<String> = if provider == foundation::Provider::LMStudio {
        state.engine.gateway().fetch_lmstudio_loaded_model().await
    } else {
        None
    };

    let gateway = state.engine.gateway();
    let prompt = foundation::Prompt {
        messages: vec![foundation::PromptMessage {
            role: "user".into(),
            content: "回复「连通测试成功」四个字".into(),
            ..Default::default()
        }],
    };
    let config = foundation::CallConfig {
        temperature: 0.0,
        max_tokens: 1024,
        stream: false,
        model: None,
        tools: None,
        tool_choice: None,
        reasoning_effort: None,
        structured_output: None,
        thinking_enabled: None,
    };
    let req = foundation::LLMRequest { provider: provider.clone(), prompt, config };

    let start = std::time::Instant::now();
    match gateway.call(&req) {
        Ok(mut rx) => {
            let mut content = String::new();
            let deadline = tokio::time::timeout(std::time::Duration::from_secs(20), async {
                while let Some(chunk_result) = rx.recv().await {
                    match chunk_result {
                        Ok(chunk) => {
                            if !chunk.content.is_empty() {
                                content.push_str(&chunk.content);
                            } else if let Some(ref rc) = chunk.reasoning_content {
                                // DeepSeek 思考模型：内容在 reasoning_content 里
                                content.push_str(rc);
                            }
                            if chunk.finish_reason.is_some() {
                                break;
                            }
                        }
                        Err(e) => {
                            content = format!("错误: {}", e);
                            break;
                        }
                    }
                }
            }).await;

            let latency = start.elapsed().as_millis() as u64;
            match deadline {
                Ok(()) => Json(TestProviderResponse {
                    ok: !content.is_empty(),
                    message: if content.is_empty() { "连接成功但未接收到内容".into() } else { content },
                    latency_ms: Some(latency),
                    model: detected_model.clone(),
                }),
                Err(_) => Json(TestProviderResponse {
                    ok: false,
                    message: "连接超时（20秒）".into(),
                    latency_ms: Some(latency),
                    model: detected_model.clone(),
                }),
            }
        }
        Err(e) => Json(TestProviderResponse {
            ok: false,
            message: format!("调用失败: {}", e),
            latency_ms: None,
            model: detected_model,
        }),
    }
}

// ── 中转站 (Agent Proxy) 配置 ──

const RELAY_CONFIG_FILE: &str = "data/relay.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayConfig {
    url: String,
    api_key: String,
}

fn load_relay_config() -> RelayConfig {
    if let Ok(content) = std::fs::read_to_string(RELAY_CONFIG_FILE) {
        if let Ok(config) = serde_json::from_str::<RelayConfig>(&content) {
            // 服务重启后恢复 env var，确保 runtime 能读到中转站配置
            if !config.api_key.is_empty() {
                std::env::set_var("AGENT_PROXY_KEY", &config.api_key);
            }
            if !config.url.is_empty() {
                std::env::set_var("AI_RELAY_URL", &config.url);
            }
            return config;
        }
    }
    RelayConfig {
        url: std::env::var("AI_RELAY_URL").unwrap_or_default(),
        api_key: std::env::var("AGENT_PROXY_KEY").unwrap_or_default(),
    }
}

fn save_relay_config(config: &RelayConfig) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Some(parent) = std::path::Path::new(RELAY_CONFIG_FILE).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(RELAY_CONFIG_FILE, json);
    }
}

lazy_static::lazy_static! {
    static ref RELAY_CONFIG: RwLock<RelayConfig> = RwLock::new(load_relay_config());
}

#[derive(Debug, Deserialize)]
struct SetRelayRequest {
    url: String,
    api_key: String,
}

#[derive(Debug, Serialize)]
struct RelayStatus {
    url: String,
    has_key: bool,
    configured: bool,
    block_prod_edit: bool,
}

async fn get_relay() -> Json<RelayStatus> {
    let cfg = read_cfg(&RELAY_CONFIG);
    let block = std::env::var("AI_RELAY_URL").ok().filter(|v| !v.is_empty()).is_some();
    Json(RelayStatus {
        url: cfg.url.clone(),
        has_key: !cfg.api_key.is_empty(),
        configured: !cfg.url.is_empty() && !cfg.api_key.is_empty(),
        block_prod_edit: block,
    })
}

async fn set_relay(
    Json(body): Json<SetRelayRequest>,
) -> Result<Json<RelayStatus>, (axum::http::StatusCode, Json<ApiError>)> {
    // 如果生产环境已通过 env var 设置，禁止前端覆盖
    if std::env::var("AI_RELAY_URL").ok().filter(|v| !v.is_empty()).is_some() {
        let cfg = read_cfg(&RELAY_CONFIG);
        return Ok(Json(RelayStatus {
            url: cfg.url.clone(),
            has_key: !cfg.api_key.is_empty(),
            configured: !cfg.url.is_empty() && !cfg.api_key.is_empty(),
            block_prod_edit: true,
        }));
    }

    let config = RelayConfig {
        url: body.url.trim().to_string(),
        api_key: body.api_key.trim().to_string(),
    };
    if !config.url.is_empty() {
        std::env::set_var("AI_RELAY_URL", &config.url);
    }
    if !config.api_key.is_empty() {
        std::env::set_var("AGENT_PROXY_KEY", &config.api_key);
    }

    let url = config.url.clone();
    let has_key = !config.api_key.is_empty();
    let configured = !config.url.is_empty() && !config.api_key.is_empty();

    save_relay_config(&config);
    *write_cfg(&RELAY_CONFIG) = config;

    Ok(Json(RelayStatus {
        url,
        has_key,
        configured,
        block_prod_edit: false,
    }))
}

async fn test_relay(
    Json(body): Json<SetRelayRequest>,
) -> Json<serde_json::Value> {
    let url = body.url.trim().trim_end_matches('/').to_string();
    let api_key = body.api_key.trim().to_string();
    let models_url = format!("{}/models", url);
    let chat_url = format!("{}/chat/completions", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("构建 reqwest client 失败");

    let start = std::time::Instant::now();

    // Test 1: GET /models
    let models_result = client
        .get(&models_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;

    // Test 2: POST /chat/completions with a minimal prompt
    let chat_result = client
        .post(&chat_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 5
        }))
        .send()
        .await;

    let latency = start.elapsed().as_millis() as u64;

    let mut models = vec![];
    if let Ok(resp) = models_result {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    models = data.iter()
                        .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                        .collect();
                }
            }
        }
    }

    let chat_ok = chat_result.map(|r| r.status().is_success()).unwrap_or(false);

    Json(serde_json::json!({
        "ok": chat_ok || !models.is_empty(),
        "models_count": models.len(),
        "models": models.iter().take(10).collect::<Vec<_>>(),
        "chat_ok": chat_ok,
        "latency_ms": latency,
    }))
}


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ProviderEndpointConfig {
    #[serde(default)]
    url: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    model: String,
}

fn load_provider_endpoint_config(path: &str) -> ProviderEndpointConfig {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str::<ProviderEndpointConfig>(&content) {
            return config;
        }
    }
    ProviderEndpointConfig::default()
}

fn save_provider_endpoint_config(path: &str, config: &ProviderEndpointConfig) {
    if let Ok(json) = serde_json::to_string_pretty(config) {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }
}

#[derive(Debug, Serialize)]
struct ProviderEndpointResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct SetProviderEndpointRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ProviderKeyResponse {
    has_key: bool,
}

#[derive(Debug, Deserialize)]
struct SetProviderKeyRequest {
    key: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProviderModelResponse {
    model: String,
}

#[derive(Debug, Deserialize)]
struct SetProviderModelRequest {
    model: String,
}

#[derive(Debug, Deserialize)]
struct ProviderTestRequest {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProviderTestResponse {
    ok: bool,
    message: String,
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    models: Option<Vec<String>>,
}

fn normalize_endpoint_url(url: &str, default: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return default.to_string();
    }
    trimmed.to_string()
}

fn test_request_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(15)
}

// ── OpenAI endpoints ──
async fn get_openai_url(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderEndpointResponse> {
    let url = state.engine.gateway().openai_base_url();
    Json(ProviderEndpointResponse { url })
}

async fn set_openai_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderEndpointRequest>,
) -> Json<ProviderEndpointResponse> {
    let url = normalize_endpoint_url(&body.url, "https://api.openai.com/v1");
    state.engine.gateway().set_openai_base_url(url.clone());
    save_provider_endpoint_config(OPENAI_CONFIG_FILE, &ProviderEndpointConfig {
        url: url.clone(),
        ..load_provider_endpoint_config(OPENAI_CONFIG_FILE)
    });
    Json(ProviderEndpointResponse { url })
}

async fn get_openai_key(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderKeyResponse> {
    let key = state.engine.gateway().openai_api_key();
    Json(ProviderKeyResponse { has_key: key.is_some() })
}

async fn set_openai_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderKeyRequest>,
) -> Json<ProviderKeyResponse> {
    let key = body.key.filter(|k| !k.is_empty());
    state.engine.gateway().set_openai_api_key(key.clone());
    save_provider_endpoint_config(OPENAI_CONFIG_FILE, &ProviderEndpointConfig {
        api_key: key.unwrap_or_default(),
        ..load_provider_endpoint_config(OPENAI_CONFIG_FILE)
    });
    let has_key = state.engine.gateway().openai_api_key().is_some();
    Json(ProviderKeyResponse { has_key })
}

async fn get_openai_model(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderModelResponse> {
    let model = state.engine.gateway().openai_model();
    Json(ProviderModelResponse { model })
}

async fn set_openai_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderModelRequest>,
) -> Json<ProviderModelResponse> {
    let model = body.model.trim().to_string();
    state.engine.gateway().set_openai_model(model.clone());
    save_provider_endpoint_config(OPENAI_CONFIG_FILE, &ProviderEndpointConfig {
        model: model.clone(),
        ..load_provider_endpoint_config(OPENAI_CONFIG_FILE)
    });
    Json(ProviderModelResponse { model })
}

async fn test_openai(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProviderTestRequest>,
) -> Json<ProviderTestResponse> {
    let configured = load_provider_endpoint_config(OPENAI_CONFIG_FILE);
    let base_url = normalize_endpoint_url(
        body.url.as_deref().unwrap_or(&configured.url),
        &state.engine.gateway().openai_base_url(),
    );
    let api_key = body.api_key.clone()
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| configured.api_key.clone());
    if api_key.is_empty() {
        return Json(ProviderTestResponse {
            ok: false,
            message: "OpenAI API key not configured".into(),
            latency_ms: None,
            model: None,
            models: None,
        });
    }

    let client = reqwest::Client::builder()
        .timeout(test_request_timeout())
        .build()
        .expect("构建 reqwest client 失败");
    let models_url = format!("{}/models", base_url);
    let chat_url = format!("{}/chat/completions", base_url);

    let start = std::time::Instant::now();
    let models_result = client
        .get(&models_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await;
    let chat_result = client
        .post(&chat_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": state.engine.gateway().openai_model(),
            "messages": [{"role": "user", "content": "回复「连通测试成功」四个字"}],
            "max_tokens": 32
        }))
        .send()
        .await;
    let latency = start.elapsed().as_millis() as u64;

    let mut models = Vec::new();
    let mut message = String::new();
    let mut ok = false;

    if let Ok(resp) = models_result {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                    models = data.iter()
                        .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                        .collect();
                }
            }
            if !models.is_empty() {
                ok = true;
                message = format!("拉取到 {} 个模型", models.len());
            }
        }
    }

    if let Ok(resp) = chat_result {
        if resp.status().is_success() {
            ok = true;
            if message.is_empty() {
                message = "Chat API reachable".into();
            }
        } else if message.is_empty() {
            message = format!("Chat API error {}", resp.status());
        }
    } else if message.is_empty() {
        message = "Chat API unreachable".into();
    }

    Json(ProviderTestResponse {
        ok,
        message,
        latency_ms: Some(latency),
        model: None,
        models: Some(models.into_iter().take(50).collect()),
    })
}

// ── Claude endpoints ──
async fn get_claude_url(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderEndpointResponse> {
    let url = state.engine.gateway().claude_base_url();
    Json(ProviderEndpointResponse { url })
}

async fn set_claude_url(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderEndpointRequest>,
) -> Json<ProviderEndpointResponse> {
    let url = normalize_endpoint_url(&body.url, "https://api.anthropic.com/v1");
    state.engine.gateway().set_claude_base_url(url.clone());
    save_provider_endpoint_config(CLAUDE_CONFIG_FILE, &ProviderEndpointConfig {
        url: url.clone(),
        ..load_provider_endpoint_config(CLAUDE_CONFIG_FILE)
    });
    Json(ProviderEndpointResponse { url })
}

async fn get_claude_key(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderKeyResponse> {
    let key = state.engine.gateway().claude_api_key();
    Json(ProviderKeyResponse { has_key: key.is_some() })
}

async fn set_claude_key(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderKeyRequest>,
) -> Json<ProviderKeyResponse> {
    let key = body.key.filter(|k| !k.is_empty());
    state.engine.gateway().set_claude_api_key(key.clone());
    save_provider_endpoint_config(CLAUDE_CONFIG_FILE, &ProviderEndpointConfig {
        api_key: key.unwrap_or_default(),
        ..load_provider_endpoint_config(CLAUDE_CONFIG_FILE)
    });
    let has_key = state.engine.gateway().claude_api_key().is_some();
    Json(ProviderKeyResponse { has_key })
}

async fn get_claude_model(
    State(state): State<Arc<AppState>>,
) -> Json<ProviderModelResponse> {
    let model = state.engine.gateway().claude_model();
    Json(ProviderModelResponse { model })
}

async fn set_claude_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetProviderModelRequest>,
) -> Json<ProviderModelResponse> {
    let model = body.model.trim().to_string();
    state.engine.gateway().set_claude_model(model.clone());
    save_provider_endpoint_config(CLAUDE_CONFIG_FILE, &ProviderEndpointConfig {
        model: model.clone(),
        ..load_provider_endpoint_config(CLAUDE_CONFIG_FILE)
    });
    Json(ProviderModelResponse { model })
}

async fn test_claude(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ProviderTestRequest>,
) -> Json<ProviderTestResponse> {
    let configured = load_provider_endpoint_config(CLAUDE_CONFIG_FILE);
    let base_url = normalize_endpoint_url(
        body.url.as_deref().unwrap_or(&configured.url),
        &state.engine.gateway().claude_base_url(),
    );
    let api_key = body.api_key.clone()
        .filter(|k| !k.is_empty())
        .unwrap_or_else(|| configured.api_key.clone());
    if api_key.is_empty() {
        return Json(ProviderTestResponse {
            ok: false,
            message: "Claude API key not configured".into(),
            latency_ms: None,
            model: None,
            models: None,
        });
    }

    let model = state.engine.gateway().claude_model();
    let client = reqwest::Client::builder()
        .timeout(test_request_timeout())
        .build()
        .expect("构建 reqwest client 失败");
    let url = format!("{}/messages", base_url);

    let start = std::time::Instant::now();
    let result = client
        .post(&url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 32,
            "messages": [{"role": "user", "content": "回复「连通测试成功」四个字"}]
        }))
        .send()
        .await;
    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) if resp.status().is_success() => Json(ProviderTestResponse {
            ok: true,
            message: "Claude endpoint reachable".into(),
            latency_ms: Some(latency),
            model: Some(model),
            models: None,
        }),
        Ok(resp) => {
            let body_text = resp.text().await.unwrap_or_default();
            Json(ProviderTestResponse {
                ok: false,
                message: format!("Claude API error: {}", body_text),
                latency_ms: Some(latency),
                model: Some(model),
                models: None,
            })
        }
        Err(err) => Json(ProviderTestResponse {
            ok: false,
            message: format!("Claude request failed: {}", err),
            latency_ms: Some(latency),
            model: Some(model),
            models: None,
        }),
    }
}

// ── Domain Profile (领域模式切换) ──

/// 内置领域预设——profile 名 → 文件名
const DOMAIN_FILES: &[(&str, &str, &str)] = &[
    // (profile_id, display_label, config_file)
    ("court", "模拟仲裁庭", "domain.court.yaml"),
    ("business", "商业沙盘推演", "domain.business.yaml"),
];

#[derive(Debug, Clone, Serialize)]
pub struct DomainOption {
    pub profile: String,
    pub label: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainInfo {
    pub profile: String,
    pub system_name: String,
    pub agent_noun: String,
    pub synthesis_verb: String,
    pub dimensions: Vec<String>,
    pub available: Vec<DomainOption>,
    pub enabled_modes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetDomainRequest {
    pub profile: String,
}

/// 推断当前领域 profile id —— 根据 system_name 术语反查
fn detect_current_profile(system_name: &str) -> String {
    for (id, label, _) in DOMAIN_FILES {
        if system_name == *label {
            return id.to_string();
        }
    }
    "custom".to_string()
}

async fn get_domain_info(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DomainInfo>, (axum::http::StatusCode, Json<ApiError>)> {
    let domain = state.engine.domain();

    let available: Vec<DomainOption> = DOMAIN_FILES
        .iter()
        .map(|(id, label, file)| {
            let path = format!("config/{}", file);
            DomainOption {
                profile: id.to_string(),
                label: label.to_string(),
                available: std::path::Path::new(&path).exists(),
            }
        })
        .collect();

    Ok(Json(DomainInfo {
        profile: detect_current_profile(&domain.terms.get("system_name").cloned().unwrap_or_default()),
        system_name: domain.terms.get("system_name").cloned().unwrap_or_else(|| "模拟仲裁庭".into()),
        agent_noun: domain.terms.get("agent_noun").cloned().unwrap_or_else(|| "庭审参与者".into()),
        synthesis_verb: domain.terms.get("synthesis_verb").cloned().unwrap_or_else(|| "法庭裁决".into()),
        dimensions: domain.coordinate.dimensions.iter().map(|d| d.name.clone()).collect(),
        available,
        enabled_modes: domain.enabled_modes.clone(),
    }))
}

async fn set_domain(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetDomainRequest>,
) -> Result<Json<DomainInfo>, (axum::http::StatusCode, Json<ApiError>)> {
    // 找到对应预设的配置文件
    let (_, _, file) = DOMAIN_FILES
        .iter()
        .find(|(id, _, _)| *id == body.profile.as_str())
        .ok_or_else(|| {
            (axum::http::StatusCode::BAD_REQUEST, Json(ApiError {
                error: format!("Unknown domain profile: {}", body.profile),
            }))
        })?;

    let source_path = format!("config/{}", file);

    // 读取领域配置内容
    let content = std::fs::read_to_string(&source_path).map_err(|e| {
        (axum::http::StatusCode::NOT_FOUND, Json(ApiError {
            error: format!("Domain config not found ({}): {}", source_path, e),
        }))
    })?;

    // 解析为 DomainProfile
    let mut profile = serde_yaml::from_str::<foundation::DomainProfile>(&content).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError {
            error: format!("Failed to parse domain config: {}", e),
        }))
    })?;

    // 渲染术语占位符
    profile.synthesis_system_prompt = profile.render(&profile.synthesis_system_prompt);
    profile.collect_system_intro = profile.render(&profile.collect_system_intro);

    // 写入 config/domain.yaml（持久化）
    std::fs::write("config/domain.yaml", &content).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(ApiError {
            error: format!("Failed to write domain.yaml: {}", e),
        }))
    })?;

    // 热重载到引擎
    state.engine.reload_domain(profile);

    tracing::info!("Domain switched to: {}", body.profile);

    // 返回新状态
    let domain = state.engine.domain();
    let available: Vec<DomainOption> = DOMAIN_FILES
        .iter()
        .map(|(id, label, f)| {
            let path = format!("config/{}", f);
            DomainOption {
                profile: id.to_string(),
                label: label.to_string(),
                available: std::path::Path::new(&path).exists(),
            }
        })
        .collect();

    Ok(Json(DomainInfo {
        profile: body.profile,
        system_name: domain.terms.get("system_name").cloned().unwrap_or_else(|| "模拟仲裁庭".into()),
        agent_noun: domain.terms.get("agent_noun").cloned().unwrap_or_else(|| "庭审参与者".into()),
        synthesis_verb: domain.terms.get("synthesis_verb").cloned().unwrap_or_else(|| "法庭裁决".into()),
        dimensions: domain.coordinate.dimensions.iter().map(|d| d.name.clone()).collect(),
        available,
        enabled_modes: domain.enabled_modes.clone(),
    }))
}
