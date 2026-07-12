mod auth;
mod bing;
mod business_sandbox;
mod coding_tools;
mod collector;
mod error;
mod middleware;
mod ocr;
mod rate_limiter;
mod routes;
mod state;
mod worker_tools;
mod store;
mod web_fetch_tool;
mod web_search_tool;
mod ws;

use std::sync::Arc;

use archive::ArchiveSystem;
use possession::PossessionEngine;
use registry::SoulRegistry;

use crate::collector::SoulCollector;
use crate::rate_limiter::RateLimiter;
use crate::state::{AppState, BusinessSandboxState};
use crate::store::AppStore;
use crate::web_fetch_tool::WebFetchTool;
use crate::web_search_tool::WebSearchTool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(false)
        .init();

    // 检测是否运行在主流云平台：请求 ipinfo.io 提取 org 字段比对已知云厂商
    // 失败或超时时静默跳过，不阻塞启动
    let _ = async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build();
        let client = match client {
            Ok(c) => c,
            Err(_) => return,
        };
        if let Ok(resp) = client.get("https://ipinfo.io/json").send().await {
            if let Ok(body) = resp.text().await {
                let lower = body.to_lowercase();
                let is_cloud = lower.contains("\"org\":")
                    && (lower.contains("amazon") || lower.contains("aws")
                        || lower.contains("google cloud") || lower.contains("gcp")
                        || lower.contains("microsoft") || lower.contains("azure")
                        || lower.contains("alibaba") || lower.contains("aliyun")
                        || lower.contains("tencent cloud") || lower.contains("huawei cloud")
                        || lower.contains("oracle cloud") || lower.contains("digitalocean")
                        || lower.contains("cloudflare"));
                if is_cloud {
                    eprintln!("  [!] 检测到云平台部署环境。");
                    eprintln!("  本软件被设计用于服务被剥夺者的集体行动。");
                    eprintln!("  你确定你的使用场景符合这个目的吗？");
                    eprintln!();
                    tracing::warn!("Cloud platform deployment detected — moral reminder printed");
                }
            }
        }
    }.await;

    tracing::info!("Loading configuration...");
    let config = foundation::Config::load()?;
    let server_port = config.server_port;

    // Validate SearXNG URL to prevent SSRF
    {
        let searxng_url = &config.searxng_url;
        if let Ok(parsed) = url::Url::parse(searxng_url) {
            let host = parsed.host_str().unwrap_or("");
            let is_loopback = host == "localhost" || host == "127.0.0.1" || host == "::1";
            let is_private = host.starts_with("192.168.") || host.starts_with("10.") || host.starts_with("172.16.");
            if !is_loopback && !is_private {
                tracing::warn!("SearXNG URL {} is not a local/private address — potential SSRF risk", host);
            }
        }
    }

    let rate_limiter = load_rate_limiter();

    // 启动限流器过期 bucket 清理定时任务
    {
        let rl = rate_limiter.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                rl.cleanup();
            }
        });
    }

    tracing::info!("Initializing store...");
    let data_dir = &config.data_dir;
    let data_dir_str = data_dir.to_str().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("data_dir 路径包含非 UTF-8 字符: {:?}", data_dir))
    })?;
    let store = Arc::new(AppStore::new(data_dir_str)?);

    tracing::info!("Loading soul registry...");
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);

    tracing::info!("Initializing AI gateway...");
    let gateway = {
        let gateway = ai_gateway::GatewayRegistry::new();
        tracing::info!("Initializing LLM cache...");
        let cache = Arc::new(ai_gateway::cache::LlMCache::new(store.db(), 3600));
        gateway.set_cache(cache);
        Arc::new(gateway)
    };

    // Load persisted LM Studio config
    {
        let lmstudio_config_file = "data/lmstudio.json";
        if let Ok(content) = std::fs::read_to_string(lmstudio_config_file) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(url) = cfg["url"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_lmstudio_base_url(url.to_string());
                    tracing::info!("Restored LM Studio URL from config: {}", url);
                }
                if let Some(key) = cfg["api_key"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_lmstudio_api_key(Some(key.to_string()));
                    tracing::info!("Restored LM Studio API key from config");
                }
                if let Some(model) = cfg["model"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_lmstudio_model(model.to_string());
                    tracing::info!("Restored LM Studio model from config: {}", model);
                }
            }
        }
    }

    // Load persisted OpenAI config
    {
        let openai_config_file = "data/openai.json";
        if let Ok(content) = std::fs::read_to_string(openai_config_file) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(url) = cfg["url"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_openai_base_url(url.to_string());
                    tracing::info!("Restored OpenAI base URL from config: {}", url);
                }
                if let Some(key) = cfg["api_key"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_openai_api_key(Some(key.to_string()));
                    tracing::info!("Restored OpenAI API key from config");
                }
                if let Some(model) = cfg["model"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_openai_model(model.to_string());
                    tracing::info!("Restored OpenAI model from config: {}", model);
                }
            }
        }
    }

    // Load persisted Claude config
    {
        let claude_config_file = "data/claude.json";
        if let Ok(content) = std::fs::read_to_string(claude_config_file) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(url) = cfg["url"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_claude_base_url(url.to_string());
                    tracing::info!("Restored Claude base URL from config: {}", url);
                }
                if let Some(key) = cfg["api_key"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_claude_api_key(Some(key.to_string()));
                    tracing::info!("Restored Claude API key from config");
                }
                if let Some(model) = cfg["model"].as_str().filter(|s| !s.is_empty()) {
                    gateway.set_claude_model(model.to_string());
                    tracing::info!("Restored Claude model from config: {}", model);
                }
            }
        }
    }

    tracing::info!("Initializing archive system...");
    let archive = Arc::new(ArchiveSystem::new(store.clone()));

    tracing::info!("Initializing possession engine...");
    let gateway_cache = gateway.get_cache();
    let mut engine = PossessionEngine::new(
        store.clone(),
        registry.clone(),
        gateway,
        config.domain.clone(),
    );

    tracing::info!("Registering built-in tools...");
    engine.tool_registry_mut().register(std::sync::Arc::new(WebSearchTool::new(
        config.searxng_url.clone(),
        config.search_engine.clone(),
    )));
    engine.tool_registry_mut().register(std::sync::Arc::new(WebFetchTool::new()));

    let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    tracing::info!("Coding tools working directory: {}", working_dir.display());
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::ReadFileTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::WriteFileTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::EditFileTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::BashCommandTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::GlobSearchTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::GrepSearchTool::new(working_dir.clone())));
    engine.tool_registry_mut().register(std::sync::Arc::new(coding_tools::ClaudeCodeTool::new(working_dir)));
    // Worker rights tools — 劳动者权益工具
    engine.tool_registry_mut().register(std::sync::Arc::new(worker_tools::CalculateSeveranceTool));
    engine.tool_registry_mut().register(std::sync::Arc::new(worker_tools::EvidenceChecklistTool));
    engine.tool_registry_mut().register(std::sync::Arc::new(worker_tools::LaborLawSearchTool::new(
        data_dir.join("knowledge").join("labor-law"),
    )));
    let engine = Arc::new(engine);

    let collector = Arc::new(SoulCollector::new(
        data_dir.to_path_buf(),
        config.searxng_url.clone(),
        config.search_engine.clone(),
    ));

    let api_token = config.api_token.clone();
    let cors_origins = config.cors_origins.clone();

    let state = Arc::new(AppState {
        registry,
        engine: engine.clone(),
        archive,
        collector,
        config: Arc::new(config),
        auto_create_tasks: Arc::new(dashmap::DashMap::new()),
        interrogation_gates: Arc::new(dashmap::DashMap::new()),
        preferred_provider: Arc::new(std::sync::RwLock::new(None)),
        api_token: api_token.clone(),
        business_sandbox: Arc::new(BusinessSandboxState::new()),
    });

    let app = build_router(state.clone(), rate_limiter, cors_origins);

    // Background cleanup tasks
    let ws_manager_for_cleanup = engine.ws_manager().clone();
    let gates_for_cleanup = state.interrogation_gates.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            ws_manager_for_cleanup.cleanup_stale_sessions();
            // Cleanup interrogation gates older than 1 hour
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            gates_for_cleanup.retain(|_, gate: &mut crate::state::InterrogationGate| {
                now.saturating_sub(gate.created_at) < 3600
            });
        }
    });

    if let Some(cache) = gateway_cache {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let _ = cache.cleanup();
            }
        });
    }

    if api_token.is_some() {
        tracing::info!("API authentication: enabled");
    } else {
        tracing::warn!("API authentication: disabled (no api_token configured)");
    }

    let bind_addr = format!("0.0.0.0:{}", server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("API server listening on http://{}", bind_addr);

    let engine_for_shutdown = engine.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutdown signal received");
        engine_for_shutdown.set_shutdown();
        tx.send(()).ok();
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            rx.await.ok();
        })
        .await?;

    Ok(())
}

fn load_rate_limiter() -> Arc<RateLimiter> {
    let settings = config::Config::builder()
        .add_source(config::File::from(std::path::Path::new("config/default.yaml")))
        .add_source(config::File::from(std::path::Path::new("config/local.yaml")).required(false))
        .build()
        .unwrap_or_else(|_| config::Config::builder().build().unwrap());

    let enabled = settings.get_bool("rate_limit.enabled").unwrap_or(true);
    if !enabled {
        tracing::info!("Rate limiter disabled");
        return Arc::new(RateLimiter::new(f64::MAX, f64::MAX));
    }

    let rps = settings.get_float("rate_limit.requests_per_second").unwrap_or(30.0);
    let burst = settings.get_float("rate_limit.burst_size").unwrap_or(60.0);
    tracing::info!("Rate limiter enabled: {:.0} req/s, burst {:.0}", rps, burst);
    Arc::new(RateLimiter::new(rps, burst))
}

fn build_router(state: Arc<AppState>, rate_limiter: Arc<RateLimiter>, cors_origins: Vec<String>) -> axum::Router {
    let api_router = routes::api_router();
    let api_token = state.api_token.clone();

    let app = axum::Router::new()
        .nest("/api/v1", api_router)
        .route(
            "/ws/possess/:session_id/:channel",
            axum::routing::get(ws::ws_handler),
        )
        .route(
            "/ws/souls/auto-create/:task_id",
            axum::routing::get(ws::auto_create_ws_handler),
        )
        .route(
            "/ws/game/:game_id",
            axum::routing::get(crate::business_sandbox::ws_handler::game_ws_handler),
        )
        .with_state(state);

    crate::middleware::apply_middleware(app, rate_limiter, api_token, cors_origins)
}
