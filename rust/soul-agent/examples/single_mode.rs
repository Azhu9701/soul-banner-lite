//! Soul Agent — Quick Start
//!
//! Demonstrates the complete setup and session flow.
//! Requires soul profiles in `data/souls/` and at least one LLM provider configured.
//!
//! Usage: cargo run -p soul-agent --example single_mode

use std::sync::Arc;

use soul_agent::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = SoulAgentConfig::from_data_dir("./data");
    println!("Data dir: {}", config.data_dir.display());

    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
    let souls = registry.list_souls(&Default::default())?;
    println!("Loaded {} souls", souls.len());
    if souls.is_empty() {
        println!("  Add .md files to data/souls/ to get started.");
        return Ok(());
    }
    for s in souls.iter().take(5) {
        println!("  - {} ({})", s.name, s.field);
    }

    let gateway = Arc::new(GatewayRegistry::new());
    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    let first = &souls[0].name;
    println!("\nStarting single session with: {}\n", first);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsEvent>(256);
    let input = PossessionInput {
        task: "请用一段话介绍你自己".into(),
        souls: vec![first.clone()],
        mode: None,
        topic: None,
        judgment: None,
        worry: None,
        unknown: None,
        interrogation_context: None,
        search_topic: false,
        search_results: None,
        task_cards: Default::default(),
    };

    match engine.start_possession(input, tx).await {
        Ok(session_id) => {
            println!("Session: {}", session_id);
            while let Some(event) = rx.recv().await {
                match event.event_type {
                    WsEventType::SoulChunk => print!("{}", event.payload),
                    WsEventType::SessionComplete => { println!("\nDone."); break; }
                    WsEventType::SoulError => eprintln!("\nError: {}", event.payload),
                    _ => {}
                }
            }
        }
        Err(e) => eprintln!("Failed: {}", e),
    }

    Ok(())
}
