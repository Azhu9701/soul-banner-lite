//! # Soul Agent
//!
//! Multi-agent AI orchestration SDK.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use soul_agent::prelude::*;
//!
//! // Create engine
//! let engine = PossessionEngine::new(store, registry, gateway, domain);
//!
//! // Start a conference session
//! let input = PossessionInput {
//!     task: "是否应该实行四天工作制？".into(),
//!     souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
//!     mode: Some("conference".into()),
//!     ..Default::default()
//! };
//! let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//! let session_id = engine.start_possession(input, tx).await?;
//! ```
//!
//! ## Architecture
//!
//! Soul Agent provides:
//! - **5 orchestration modes**: single, conference, debate, relay, learn
//! - **TopologyPlanner**: automatic orchestration strategy selection
//! - **CrossDetector**: real-time conflict detection between agents
//! - **Multi-provider LLM**: OpenAI, Claude, DeepSeek, LM Studio with automatic fallback
//! - **Ismism taxonomy**: 4-dimensional soul classification for diversity scoring

pub mod prelude {
    pub use ai_gateway::GatewayRegistry;
    pub use foundation::{CallConfig, PossessionMode, Prompt, PromptMessage, Provider};
    pub use possession::{PossessionEngine, PossessionInput};
}
