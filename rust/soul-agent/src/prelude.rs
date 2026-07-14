//! 预导入模块 — `use soul_agent::prelude::*;`

pub use ai_gateway::GatewayRegistry;
pub use foundation::{DomainProfile, SoulAgentConfig};
pub use possession::{PossessionEngine, PossessionInput, WsEvent, WsEventType};
pub use registry::SoulRegistry;

pub use crate::SoulStore;
