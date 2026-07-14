//! # Soul Agent
//!
//! Multi-agent AI orchestration SDK for Rust.
//!
//! ```rust,no_run
//! use soul_agent::prelude::*;
//! use std::sync::Arc;
//!
//! // 1. Create config
//! let config = SoulAgentConfig::from_data_dir("./data");
//!
//! // 2. Create storage
//! let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
//!
//! // 3. Load registry
//! let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
//!
//! // 4. Create LLM gateway
//! let gateway = Arc::new(GatewayRegistry::new());
//!
//! // 5. Create engine
//! let engine = PossessionEngine::new(
//!     store,
//!     registry,
//!     gateway,
//!     config.domain,
//! );
//!
//! // 6. Start a conference
//! let input = PossessionInput {
//!     task: "是否应该实行四天工作制？".into(),
//!     souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
//!     mode: Some("conference".into()),
//!     ..Default::default()
//! };
//! let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
//! let session_id = engine.start_possession(input, tx).await?;
//! ```

pub mod prelude {
    pub use ai_gateway::GatewayRegistry;
    pub use foundation::{DomainProfile, Prompt, PromptMessage, Provider};
    pub use possession::{PossessionEngine, PossessionInput, WsEvent, WsEventType};

    // Config
    pub use foundation::SoulAgentConfig;

    // Storage (re-export the soul-agent store)
    // Available via `soul_agent::store::SoulStore` for custom setups
}

/// Lightweight Storage implementation.
/// Re-exported for convenience; use `ApiStore` from `api` crate for full-featured storage.
pub mod store {
    pub use foundation::FileStore;
}
