//! Soul Agent CLI — 决策引擎命令行工具
//!
//! Usage:
//!   soul "是否应该实行四天工作制？"
//!   soul "task" --souls 经济学家,HR总监 --mode conference
//!   soul --preset business-strategy "是否进入东南亚市场"
//!   soul --tui
//!   echo "task" | soul

mod tui;

use std::io::{self, Write};
use std::sync::Arc;

use clap::Parser;
use colored::*;
use foundation::PossessionMode;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use soul_agent::prelude::*;

#[derive(Parser)]
#[command(name = "soul", about = "🧠 Soul Agent — 决策引擎")]
struct Cli {
    task: Vec<String>,

    #[arg(short, long, value_delimiter = ',')]
    souls: Vec<String>,

    #[arg(short, long, default_value = "conference")]
    mode: String,

    #[arg(long, default_value = "./data")]
    data_dir: String,

    #[arg(long)]
    preset: Option<String>,

    #[arg(long)]
    tui: bool,

    #[arg(long)]
    no_tui: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct PresetConfig {
    label: String,
    souls: Vec<String>,
    mode: String,
    #[serde(default)]
    description: String,
}

fn load_preset(name: &str) -> Option<PresetConfig> {
    let content = std::fs::read_to_string("config/presets.yaml").ok()?;
    let presets: std::collections::HashMap<String, PresetConfig> = serde_yaml::from_str(&content).ok()?;
    presets.get(name).cloned()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("warn").with_target(false).init();

    let cli = Cli::parse();

    // Load preset
    let (souls, mode) = if let Some(ref name) = cli.preset {
        match load_preset(name) {
            Some(p) => {
                eprintln!("📋 {} — {}", p.label, p.description);
                (p.souls, p.mode)
            }
            None => {
                eprintln!("{} Unknown preset: {}", "Warning:".yellow(), name);
                (cli.souls.clone(), cli.mode.clone())
            }
        }
    } else {
        (cli.souls.clone(), cli.mode.clone())
    };

    // TUI mode
    let use_tui = cli.tui || (cli.task.is_empty() && !cli.no_tui);
    if use_tui {
        let task = if cli.task.is_empty() { String::new() } else { cli.task.join(" ") };
        return tui::run(task, souls, mode, cli.data_dir).await;
    }

    // Streaming mode
    let task = if cli.task.is_empty() {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    } else {
        cli.task.join(" ")
    };

    if task.is_empty() {
        eprintln!("{} 请提供任务描述", "Error:".red().bold());
        return Ok(());
    }

    let config = SoulAgentConfig::from_data_dir(&cli.data_dir);
    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
    let gateway = Arc::new(GatewayRegistry::new());

    let final_souls: Vec<String> = if souls.is_empty() {
        let all = registry.list_souls(&Default::default())?;
        if all.len() >= 3 { all.iter().take(3).map(|s| s.name.clone()).collect() }
        else { all.iter().map(|s| s.name.clone()).collect() }
    } else { souls };

    if final_souls.is_empty() {
        eprintln!("{} No souls available", "Error:".red().bold());
        return Ok(());
    }

    let mode_enum = match mode.as_str() {
        "single" => Some(PossessionMode::Single),
        "debate" => Some(PossessionMode::Debate),
        _ => Some(PossessionMode::Conference),
    };

    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    println!();
    println!("{}", "  Soul Agent".bold());
    println!("  {}  {}", "mode:".dimmed(), mode.cyan());
    println!("  {}  {}", "souls:".dimmed(), final_souls.join(", ").green());
    println!("  {}  {}", "task:".dimmed(), task.white());
    println!();

    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    let input = PossessionInput {
        task,
        souls: final_souls,
        mode: mode_enum,
        topic: None, judgment: None, worry: None, unknown: None,
        interrogation_context: None, search_topic: false, search_results: None,
        task_cards: Default::default(),
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
    pb.set_message("preparing...");

    engine.start_possession(input, tx).await?;
    pb.finish_and_clear();

    let mut current = String::new();
    while let Some(event) = rx.recv().await {
        match event.event_type {
            WsEventType::SoulStarted => {
                let name = event.soul_name.clone().unwrap_or_default();
                if name != current {
                    if !current.is_empty() { println!(); }
                    println!("\n{}", format!("── {} ──", &name).yellow().bold());
                    current = name;
                }
            }
            WsEventType::SoulChunk => { print!("{}", event.payload); io::stdout().flush()?; }
            WsEventType::SoulDone => println!(),
            WsEventType::SynthesisChunk => {
                if current != "synthesis" {
                    current = "synthesis".into();
                    println!("\n{}", "── 决策报告 ──".cyan().bold());
                }
                print!("{}", event.payload);
                io::stdout().flush()?;
            }
            WsEventType::Collision => println!("\n{} {}", "⚡ ".red().bold(), event.payload),
            WsEventType::SessionComplete => { println!("\n"); break; }
            WsEventType::SoulError => eprintln!("\n{} {}: {}", "✗".red().bold(), event.soul_name.unwrap_or_default(), event.payload),
            _ => {}
        }
    }

    Ok(())
}
