//! Soul Agent SDK — Quick Start Example
//!
//! This example demonstrates the minimal setup to use Soul Agent as a library.
//! It shows the configuration, initialization, and session flow pattern.
//!
//! ```rust,ignore
//! // The actual implementation requires:
//! // 1. A Storage implementation (use FileStore + SqliteDb, or implement the Storage trait)
//! // 2. Soul profiles in data/souls/
//! // 3. At least one LLM provider configured (API key or LM Studio)
//! ```
//!
//! Usage: cargo run -p soul-agent --example quick_start

use foundation::SoulAgentConfig;

fn main() {
    println!("Soul Agent SDK v{}", env!("CARGO_PKG_VERSION"));
    println!();

    // 1. Create config — no YAML file needed
    let config = SoulAgentConfig::from_data_dir("./data");
    println!("Config: data_dir={}", config.data_dir.display());
    println!("  souls_dir={}", config.souls_dir.display());
    println!("  db_path={}", config.db_path.display());
    println!();

    // 2. Setup pattern (pseudocode):
    println!("Setup pattern:");
    println!("  let fs = Arc::new(FileStore::new(souls_dir, archive_dir, ...)?);");
    println!("  let db = Arc::new(SqliteDb::open(&db_path)?);");
    println!("  let store = MyStore::new(fs, db);  // impl Storage");
    println!("  let registry = Arc::new(SoulRegistry::new(store.clone()).await?);");
    println!("  let gateway = Arc::new(GatewayRegistry::new());");
    println!("  let engine = PossessionEngine::new(store, registry, gateway, config.domain);");
    println!();

    // 3. Start session (pseudocode):
    println!("Session flow:");
    println!("  let input = PossessionInput {{ task, souls, mode, ..Default::default() }};");
    println!("  let (tx, rx) = tokio::sync::mpsc::unbounded_channel();");
    println!("  let session_id = engine.start_possession(input, tx).await?;");
    println!("  while let Ok(event) = rx.recv().await {{ ... }}");
    println!();

    println!("For a working example, see the main project at https://github.com/...");
}
