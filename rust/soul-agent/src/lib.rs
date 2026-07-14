//! # Soul Agent
//!
//! Multi-agent AI orchestration SDK for Rust.
//!
//! ```rust,no_run
//! use soul_agent::prelude::*;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SoulAgentConfig::from_data_dir("./data");
//!     let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
//!     let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
//!     let gateway = Arc::new(GatewayRegistry::new());
//!     let engine = PossessionEngine::new(store, registry, gateway, config.domain);
//!
//!     let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
//!     let input = PossessionInput {
//!         task: "是否应该实行四天工作制？".into(),
//!         souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
//!         mode: Some("conference".into()),
//!         ..Default::default()
//!     };
//!     engine.start_possession(input, tx).await?;
//!     while let Ok(event) = rx.recv().await {
//!         println!("[{:?}] {}", event.event_type, event.payload);
//!     }
//!     Ok(())
//! }
//! ```

pub mod prelude;

/// 内置 Storage 实现。开箱即用，基于 FileStore + SQLite。
pub mod store;
pub use store::SoulStore;
