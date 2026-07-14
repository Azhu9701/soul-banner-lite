//! Soul Agent TUI — 全程保持终端 raw 模式，无白屏

mod app;
mod ui;

use app::App;
use ui::draw;

use std::io;
use std::sync::Arc;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Terminal;

use foundation::{PossessionMode, SoulAgentConfig};
use possession::{PossessionInput, WsEvent, WsEventType};
use crate::SoulStore;
use registry::SoulRegistry;
use ai_gateway::GatewayRegistry;
use possession::PossessionEngine;

enum TuiPhase {
    Input,
    Loading,
    Results,
}

pub async fn run(
    task: String,
    souls: Vec<String>,
    mode: String,
    data_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SoulAgentConfig::from_data_dir(&data_dir);
    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);

    let all_souls = registry.list_souls(&Default::default())?;
    let available: Vec<String> = all_souls.iter().map(|s| s.name.clone()).collect();

    let gateway = Arc::new(GatewayRegistry::new());
    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    let mode_enum = match mode.as_str() {
        "single" => Some(PossessionMode::Single),
        "debate" => Some(PossessionMode::Debate),
        _ => Some(PossessionMode::Conference),
    };

    // ── Setup TUI once, keep it alive ──
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── Phase: Input ──
    let mut task_input = task;
    let mut selected: Vec<String> = if souls.is_empty() {
        available.iter().take(3).cloned().collect()
    } else {
        souls.clone()
    };

    if task_input.is_empty() {
        loop {
            terminal.draw(|f| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3)])
                    .split(area);

                let title = Paragraph::new("🧠 Soul Agent")
                    .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan)));
                f.render_widget(title, chunks[0]);

                let display = if task_input.is_empty() {
                    "在此输入你的问题...".to_string()
                } else {
                    format!("{}█", task_input)
                };
                f.render_widget(
                    Paragraph::new(Text::from(display))
                        .block(Block::default().borders(Borders::ALL).title(" 问题 ").style(Style::default().fg(Color::Yellow)))
                        .wrap(Wrap { trim: false }),
                    chunks[1],
                );

                let footer_text = format!(
                    "Souls: {} | Mode: {} | Enter:提交  Tab:换Soul  q:退出",
                    selected.join("、"), mode
                );
                f.render_widget(
                    Paragraph::new(Text::from(footer_text))
                        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::DarkGray))),
                    chunks[2],
                );
            })?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            terminal.backend_mut().execute(LeaveAlternateScreen)?;
                            return Ok(());
                        }
                        KeyCode::Enter => {
                            if !task_input.is_empty() { break; }
                        }
                        KeyCode::Tab => {
                            if available.len() > 3 {
                                let start = ((selected.len() / 3) * 3) % available.len();
                                selected = available.iter().skip(start).take(3).cloned().collect();
                            }
                        }
                        KeyCode::Char(c) => task_input.push(c),
                        KeyCode::Backspace => { task_input.pop(); }
                        _ => {}
                    }
                }
            }
        }
    }

    // ── Phase: Loading ──
    terminal.draw(|f| {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(area);

        f.render_widget(
            Paragraph::new("🧠 Soul Agent")
                .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan))),
            chunks[0],
        );

        let msg = format!(
            "正在召唤 {} ...\n\n问题: {}\n\n⏳ 请稍候，AI 正在分析和回应...",
            selected.join("、"), task_input
        );
        f.render_widget(
            Paragraph::new(Text::from(msg))
                .block(Block::default().borders(Borders::ALL).title(" 运行中 "))
                .style(Style::default().fg(Color::Yellow)),
            chunks[1],
        );
    })?;

    // Start session
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsEvent>(256);
    let input = PossessionInput {
        task: task_input.clone(),
        souls: selected.clone(),
        mode: mode_enum,
        topic: None, judgment: None, worry: None, unknown: None,
        interrogation_context: None, search_topic: false, search_results: None,
        task_cards: Default::default(),
    };

    engine.start_possession(input, tx).await?;

    let mut events: Vec<WsEvent> = Vec::new();
    while let Some(event) = rx.recv().await {
        events.push(event);
    }

    // ── Phase: Results ──
    let mut app = App::new(task_input, selected, mode, events);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.prev_tab(),
                    KeyCode::Up => app.scroll_up(),
                    KeyCode::Down => app.scroll_down(),
                    KeyCode::Left => app.prev_round(),
                    KeyCode::Right => app.next_round(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok(())
}
