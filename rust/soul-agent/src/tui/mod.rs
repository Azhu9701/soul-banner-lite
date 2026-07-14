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
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
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
    let mut soul_cursor: usize = 0;
    let mut focus_input = true; // true = typing in task field, false = navigating souls

    if task_input.is_empty() {
        loop {
            terminal.draw(|f| {
                let area = f.area();
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3)])
                    .split(area);

                f.render_widget(
                    Paragraph::new("🧠 Soul Agent")
                        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan))),
                    chunks[0],
                );

                // Main area: soul list + input
                let main = chunks[1];
                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(main);

                // Soul list (left)
                let mut soul_lines: Vec<Line> = Vec::new();
                soul_lines.push(Line::from(Span::styled(
                    format!("{} 可用 Soul ({})", if focus_input { "  " } else { "▶ " }, available.len()),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                for (i, name) in available.iter().enumerate() {
                    let checked = selected.contains(name);
                    let marker = if checked { "[✓]" } else { "[ ]" };
                    let cursor = if !focus_input && i == soul_cursor { " ▸" } else { "  " };
                    let style = if !focus_input && i == soul_cursor {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else if checked {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    soul_lines.push(Line::from(Span::styled(
                        format!("{} {}{}", cursor, marker, name),
                        style,
                    )));
                }
                f.render_widget(
                    Paragraph::new(Text::from(soul_lines))
                        .block(Block::default().borders(Borders::ALL).title(" Souls "))
                        .wrap(Wrap { trim: false }),
                    main_chunks[0],
                );

                // Task input (right)
                let input_style = if focus_input {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let display = if task_input.is_empty() {
                    "在此输入问题...".to_string()
                } else {
                    format!("{}█", task_input)
                };
                f.render_widget(
                    Paragraph::new(Text::from(display))
                        .block(Block::default().borders(Borders::ALL).title(" 问题 ").style(input_style))
                        .wrap(Wrap { trim: false }),
                    main_chunks[1],
                );

                // Footer
                let selected_display = if selected.is_empty() {
                    "无".to_string()
                } else {
                    selected.join("、")
                };
                let footer_text = format!(
                    "已选: {} | Mode: {} | Tab:切换面板  Space:选中  ↑↓:导航  Enter:提交  q:退出",
                    selected_display, mode
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
                            if !task_input.is_empty() && !selected.is_empty() { break; }
                        }
                        KeyCode::Tab => focus_input = !focus_input,
                        KeyCode::Up if !focus_input => {
                            soul_cursor = soul_cursor.saturating_sub(1);
                        }
                        KeyCode::Down if !focus_input => {
                            soul_cursor = (soul_cursor + 1).min(available.len().saturating_sub(1));
                        }
                        KeyCode::Char(' ') if !focus_input => {
                            if let Some(name) = available.get(soul_cursor) {
                                if selected.contains(name) {
                                    selected.retain(|s| s != name);
                                } else {
                                    selected.push(name.clone());
                                }
                            }
                        }
                        KeyCode::Char(c) if focus_input => task_input.push(c),
                        KeyCode::Backspace if focus_input => { task_input.pop(); }
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
        mode: mode_enum.clone(),
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
    let mut app = App::new(task_input, selected, mode.clone(), events);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                if app.follow_up_active {
                    match key.code {
                        KeyCode::Esc => app.toggle_follow_up(),
                        KeyCode::Enter => {
                            let question = app.follow_up_input.clone();
                            if !question.is_empty() {
                                app.toggle_follow_up();
                                // Run follow-up session
                                let (tx2, mut rx2) = tokio::sync::mpsc::channel::<WsEvent>(256);
                                let input2 = PossessionInput {
                                    task: question,
                                    souls: app.souls.clone(),
                                    mode: mode_enum.clone(),
                                    topic: None, judgment: None, worry: None, unknown: None,
                                    interrogation_context: None, search_topic: false, search_results: None,
                                    task_cards: Default::default(),
                                };
                                if engine.start_possession(input2, tx2).await.is_ok() {
                                    let mut new_events = Vec::new();
                                    while let Some(event) = rx2.recv().await {
                                        new_events.push(event);
                                    }
                                    app = App::new(app.task.clone(), app.souls.clone(), app.mode.clone(), new_events);
                                }
                            }
                        }
                        KeyCode::Char(c) => app.follow_up_push(c),
                        KeyCode::Backspace => app.follow_up_pop(),
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => app.toggle_follow_up(),
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
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok(())
}
