# Soul Agent

**Multi-agent AI orchestration SDK for Rust.**

```bash
cargo add soul-agent
```

## Quick Start (5 lines)

```rust
use soul_agent::prelude::*;
use std::sync::Arc;
use foundation::SoulAgentConfig;

let config = SoulAgentConfig::from_data_dir("./data");
let store = Arc::new(/* impl Storage */);
let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
let gateway = Arc::new(GatewayRegistry::new());
let engine = PossessionEngine::new(store, registry, gateway, config.domain);
```

## Architecture

```
soul-agent (SDK)
  ├── PossessionEngine     ← multi-agent conference/debate/single/relay/learn
  ├── GatewayRegistry      ← multi-provider LLM (OpenAI/Claude/DeepSeek/LM Studio)
  ├── SoulAgentConfig      ← minimal SDK config (no YAML/env needed)
  └── prelude              ← convenience re-exports
```

## Orchestration Modes

| Mode | When |
|------|------|
| `single` | Simple Q&A with one soul |
| `conference` | Multi-soul parallel + synthesis |
| `debate` | Two opposing souls |
| `relay` | Sequential soul chain |
| `learn` | Teaching/learning mode |

## Examples

```bash
cargo run -p soul-agent --example single_mode
cargo run -p soul-agent --example conference_mode
```

## License

MIT
