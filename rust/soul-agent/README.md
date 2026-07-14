# Soul Agent

**Multi-agent AI orchestration platform.** SDK + HTTP service.

```bash
cargo add soul-agent
```

## Quick Start

```rust
use soul_agent::prelude::*;

let engine = PossessionEngine::new(store, registry, gateway, domain);
let input = PossessionInput {
    task: "是否应该实行四天工作制？".into(),
    souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
    mode: Some("conference".into()),
    ..Default::default()
};
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
let session_id = engine.start_possession(input, tx).await?;
```

## Architecture

```
soul-agent (SDK) ─── wraps ─── possession + ai-gateway
     │
     └── prelude: PossessionEngine, PossessionInput, GatewayRegistry, ...
```

## Components

- [possession](../possession/README.md) — multi-agent orchestration engine
- [ai-gateway](../ai-gateway/README.md) — multi-provider LLM gateway
- [api](../api/) — HTTP API server
