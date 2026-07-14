use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use archive::ArchiveSystem;
use dashmap::DashMap;
use foundation::Config;
use possession::PossessionEngine;
use registry::SoulRegistry;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;

use crate::collector::SoulCollector;
use crate::searxng_cache::SearxngCache;

use crate::business_sandbox::engine::GameManager;

#[derive(Debug, Clone, Serialize)]
pub struct AutoCreateEvent {
    pub task_id: String,
    pub soul_name: String,
    pub phase: String, // "collecting" | "refining" | "done" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<foundation::SoulProfile>,
}

/// 审查官入场审讯 — 在合议前拦截使用者，判断是否"以此享乐"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterrogationQuestion {
    pub text: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct InterrogationGate {
    pub task: String,
    pub questions: Vec<InterrogationQuestion>,
    pub created_at: u64,
}

pub struct BusinessSandboxState {
    pub manager: GameManager,
    ws_senders: AsyncMutex<HashMap<String, Vec<mpsc::Sender<String>>>>,
}

impl BusinessSandboxState {
    pub fn new() -> Self {
        Self {
            manager: GameManager::new(),
            ws_senders: AsyncMutex::new(HashMap::new()),
        }
    }

    pub async fn register_ws(&self, game_id: &str, tx: mpsc::Sender<String>) {
        let mut senders = self.ws_senders.lock().await;
        senders.entry(game_id.to_string()).or_default().push(tx);
    }

    pub async fn broadcast(&self, game_id: &str, message: &str) {
        let mut senders = self.ws_senders.lock().await;
        if let Some(entries) = senders.get_mut(game_id) {
            entries.retain(|tx| tx.try_send(message.to_string()).is_ok());
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<SoulRegistry>,
    pub engine: Arc<PossessionEngine>,
    pub archive: Arc<ArchiveSystem>,
    pub collector: Arc<SoulCollector>,
    pub config: Arc<Config>,
    pub auto_create_tasks: Arc<DashMap<String, broadcast::Sender<AutoCreateEvent>>>,
    pub interrogation_gates: Arc<DashMap<String, InterrogationGate>>,
    pub preferred_provider: Arc<RwLock<Option<foundation::Provider>>>,
    pub api_token: Option<String>,
    pub business_sandbox: Arc<BusinessSandboxState>,
    pub searxng_cache: Arc<SearxngCache>,
}
