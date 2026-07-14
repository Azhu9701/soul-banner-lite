# Soul Agent — 外部接入指南

## 安装

### Git 依赖（当前）

```toml
[dependencies]
soul-agent = { git = "https://github.com/your-org/rust-agent", package = "soul-agent" }
```

### 发布到 crates.io 后

```toml
[dependencies]
soul-agent = "0.1"
```

## 5 行启动

```rust
use soul_agent::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 配置
    let config = SoulAgentConfig::from_data_dir("./data");

    // 2. 存储
    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);

    // 3. 注册表
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);

    // 4. LLM 网关
    let gateway = Arc::new(GatewayRegistry::new());

    // 5. 引擎
    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    // 启动会话
    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    let input = PossessionInput {
        task: "是否应该实行四天工作制？".into(),
        souls: vec!["经济学家".into(), "HR总监".into(), "工会代表".into()],
        ..PossessionInput::default_fields()
    };
    engine.start_possession(input, tx).await?;

    while let Some(event) = rx.recv().await {
        println!("[{:?}] {}", event.event_type, event.payload);
    }
    Ok(())
}
```

## 运行示例

```bash
# 查看 setup 模式
cargo run -p soul-agent --example single_mode

# 自己写代码参考
# examples/single_mode.rs
```

## Soul 文件格式

放在 `data/souls/` 目录下，`.md` 格式：

```markdown
---
name: 经济学家
field: 经济学
ismism_code: "1-2-1-0"
ontology: 唯物主义
epistemology: 经验主义
teleology: 功利主义
self_declare: 我是一位宏观经济学家，擅长供给侧分析
trigger_keywords: ["经济", "市场", "通胀", "GDP"]
---

## 召唤词

你是一位宏观经济学家...
```

## 运行模式

| mode | soul 数 | 说明 |
|------|---------|------|
| `single` | 1 | 单角色回答 |
| `conference` | 2+ | 多角色并行 + 综合 |
| `debate` | 2 | 双角色辩论 |
| `relay` | 2+ | 串行接力 |

## 多 Provider

设置环境变量即可：

```bash
export DEEPSEEK_API_KEY=sk-xxx
export OPENAI_API_KEY=sk-xxx
export ANTHROPIC_API_KEY=sk-xxx
```

Gateway 自动选择可用的 provider，失败时自动切换。

## 自定义存储

实现 `foundation::Storage` trait 即可替换默认 SoulStore：

```rust
struct MyStore { /* your backend */ }

#[async_trait]
impl Storage for MyStore {
    // 实现所有方法，或只实现核心方法（其余返回 Ok/空）
}
```
