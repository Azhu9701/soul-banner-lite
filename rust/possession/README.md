# Possession SDK

**Multi-agent AI orchestration for Rust.** Part of the [Soul Agent](https://github.com/earendil-works/pi) platform.

5 lines of code to start a multi-agent conference:

```rust
use possession::{PossessionEngine, PossessionInput};

let engine = PossessionEngine::new(store, registry, gateway, domain);

let input = PossessionInput {
    task: "是否应该实行四天工作制？".into(),
    souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
    mode: Some("conference".into()),
    ..Default::default()
};

let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
let session_id = engine.start_possession(input, tx).await?;

// Stream real-time events: each soul's output, collisions, synthesis
while let Ok(event) = rx.recv().await {
    println!("[{:?}] {}: {}", event.event_type, event.soul_name.unwrap_or_default(), event.payload);
}
```

## Orchestration Modes

| Mode | Description | When to use |
|------|-------------|------------|
| `single` | Single soul responds | Simple Q&A |
| `conference` | Multi-soul parallel + synthesis | Complex analysis needing diverse views |
| `debate` | Two opposing souls | Polarized topics |
| `relay` | Sequential soul chain | Multi-stage workflows |
| `learn` | Teaching/learning mode | Knowledge transfer |

### Automatic Topology Selection

The **TopologyPlanner** automatically selects the optimal orchestration strategy:

```
Task complexity + Soul diversity → Topology:
  Low complexity + budget   → Minimal (1 soul, cheapest)
  High diversity + complex  → FullMesh + cross_detect
  2 souls                   → Oppositional (debate)
  Default                   → ClusteredParallel
```

## Real-Time Cross-Detection

The **CrossDetector** monitors streaming output from all souls in real-time, detecting:

- **Contradictions**: 但是、然而、不同意...
- **Perspective Differences**: 从另一个角度、换个视角...
- **Premise Disagreements**: 假设、前提分歧...
- **Supplementary Challenges**: 补充、需要注意...

When a collision is detected, it can be injected into the conversation — prompting souls to respond to each other's points.

## Multi-Provider LLM

Four providers with automatic fallback:

```rust
let gateway = ai_gateway::GatewayRegistry::new();
gateway.set_cache(Arc::new(LlMCache::new(db, 3600))); // SQLite cache

// Automatic provider switching on failure
let provider = gateway.pick_provider(); // picks healthiest available
let rx = gateway.call(&LLMRequest { provider, prompt, config })?;
```

## API Reference

- `PossessionEngine` — main entry point
- `PossessionInput` — session configuration
- `WsEvent` / `WsEventType` — streaming event types
- `TopologyPlanner` — automatic orchestration strategy
- `CrossDetector` — real-time conflict detection
- `ToolRegistry` / `ToolHandler` — tool registration and execution
- `GatewayRegistry` — multi-provider LLM gateway
