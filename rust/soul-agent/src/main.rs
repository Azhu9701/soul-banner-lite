//! Soul Agent CLI — 多智能体命令行工具
//!
//! Usage:
//!   soul "是否应该实行四天工作制？"
//!   soul "task" --souls 经济学家,HR总监 --mode conference
//!   soul "task" --tui
//!   echo "task" | soul

mod tui;

use std::io::{self, Write};
use std::sync::Arc;

use clap::Parser;
use colored::*;
use foundation::PossessionMode;
use indicatif::{ProgressBar, ProgressStyle};
use soul_agent::prelude::*;

// ── CLI ────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "soul",
    about = "🧠 Soul Agent — 多智能体 AI 编排 CLI",
    long_about = "多角色 AI 对话工具。\n\n\
                  支持 conference（合议）、debate（辩论）、single（单角色）。\n\
                  自动流式输出、交叉检测、综合裁决。"
)]
struct Cli {
    /// 任务描述
    task: Vec<String>,

    /// 指定 soul（逗号分隔）
    #[arg(short, long, value_delimiter = ',')]
    souls: Vec<String>,

    /// 运行模式: single, conference, debate
    #[arg(short, long, default_value = "conference")]
    mode: String,

    /// 数据目录
    #[arg(long, default_value = "./data")]
    data_dir: String,

    /// 启动 TUI 模式
    #[arg(long)]
    tui: bool,

    /// 非交互模式（纯流式输出，不启动 TUI）
    #[arg(long)]
    no_tui: bool,
}

// ── Main ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // ── TUI mode (default when no task or --tui flag) ──
    let use_tui = cli.tui || (cli.task.is_empty() && !cli.no_tui);
    if use_tui {
        let task = if cli.task.is_empty() {
            String::new() // TUI will show input prompt
        } else {
            cli.task.join(" ")
        };
        return tui::run(task, cli.souls, cli.mode, cli.data_dir).await;
    }

    // ── Streaming mode ──
    let task = if cli.task.is_empty() {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    } else {
        cli.task.join(" ")
    };

    if task.is_empty() {
        eprintln!("{} 请提供任务描述", "Error:".red().bold());
        eprintln!("  soul \"你的问题\"");
        eprintln!("  echo \"你的问题\" | soul");
        return Ok(());
    }

    let mode = match cli.mode.as_str() {
        "single" => Some(PossessionMode::Single),
        "debate" => Some(PossessionMode::Debate),
        _ => Some(PossessionMode::Conference),
    };

    // ── Streaming mode ──
    // Init SDK
    let config = SoulAgentConfig::from_data_dir(&cli.data_dir);
    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);
    let gateway = Arc::new(GatewayRegistry::new());

    // Resolve souls BEFORE engine consumes registry
    let souls: Vec<String> = if cli.souls.is_empty() {
        let all = registry.list_souls(&Default::default())?;
        if all.len() >= 3 {
            all.iter().take(3).map(|s| s.name.clone()).collect()
        } else {
            all.iter().map(|s| s.name.clone()).collect()
        }
    } else {
        cli.souls.clone()
    };

    if souls.is_empty() {
        eprintln!("{} 没有可用 soul。将 .md 文件放入 {}/souls/", "Error:".red().bold(), cli.data_dir);
        return Ok(());
    }

    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    // Header
    let mode_str = cli.mode;
    println!();
    println!("{}", "  Soul Agent".bold());
    println!("  {}  {}", "mode:".dimmed(), mode_str.cyan());
    println!("  {}  {}", "souls:".dimmed(), souls.join(", ").green());
    println!("  {}  {}", "task:".dimmed(), task.white());
    println!();

    // Run
    let (tx, mut rx) = tokio::sync::mpsc::channel(256);
    let input = PossessionInput {
        task,
        souls,
        mode,
        topic: None,
        judgment: None,
        worry: None,
        unknown: None,
        interrogation_context: None,
        search_topic: false,
        search_results: None,
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
            WsEventType::SoulChunk => {
                print!("{}", event.payload);
                io::stdout().flush()?;
            }
            WsEventType::SoulDone => println!(),
            WsEventType::SynthesisChunk => {
                if current != "synthesis" {
                    current = "synthesis".into();
                    println!("\n{}", "── 综合裁决 ──".cyan().bold());
                }
                print!("{}", event.payload);
                io::stdout().flush()?;
            }
            WsEventType::Collision => {
                println!("\n{} {}", "⚡ 碰撞:".red().bold(), event.payload);
            }
            WsEventType::SessionComplete => { println!("\n"); break; }
            WsEventType::SoulError => {
                eprintln!("\n{} {}: {}", "✗".red().bold(), event.soul_name.unwrap_or_default(), event.payload);
            }
            _ => {}
        }
    }

    Ok(())
}
