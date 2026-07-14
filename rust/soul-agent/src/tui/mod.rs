//! Soul Agent TUI — 质量优先的多面板终端界面

mod app;
mod ui;

use app::App;
use ui::draw;

use std::io;
use std::sync::Arc;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use foundation::{PossessionMode, SoulAgentConfig};
use possession::{PossessionInput, WsEvent};
use crate::SoulStore;
use registry::SoulRegistry;
use ai_gateway::GatewayRegistry;
use possession::PossessionEngine;

pub async fn run(
    task: String,
    souls: Vec<String>,
    mode: String,
    data_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = SoulAgentConfig::from_data_dir(&data_dir);
    let store = Arc::new(SoulStore::new(config.data_dir.to_str().unwrap())?);
    let registry = Arc::new(SoulRegistry::new(store.clone()).await?);

    // If no task given, enter input mode
    let (final_task, final_souls) = if task.is_empty() {
        run_input_mode(&registry, &mode, &souls).await?
    } else {
        let resolved_souls = if souls.is_empty() {
            let all = registry.list_souls(&Default::default())?;
            if all.len() >= 3 { all.iter().take(3).map(|s| s.name.clone()).collect() }
            else { all.iter().map(|s| s.name.clone()).collect() }
        } else { souls.clone() };
        (task, resolved_souls)
    };

    if final_task.is_empty() {
        return Ok(());
    }

    let gateway = Arc::new(GatewayRegistry::new());
    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    let mode_enum = match mode.as_str() {
        "single" => Some(PossessionMode::Single),
        "debate" => Some(PossessionMode::Debate),
        _ => Some(PossessionMode::Conference),
    };

    // Start session
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsEvent>(256);
    let input = PossessionInput {
        task: final_task.clone(),
        souls: final_souls.clone(),
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

    // TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(final_task, final_souls, mode, events);

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

/// Input mode: user types question and selects souls in the terminal
async fn run_input_mode(
    registry: &SoulRegistry,
    mode: &str,
    preset_souls: &[String],
) -> Result<(String, Vec<String>), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let all_souls = registry.list_souls(&Default::default())?;
    let available: Vec<String> = all_souls.iter().map(|s| s.name.clone()).collect();

    let mut selected: Vec<String> = if preset_souls.is_empty() {
        available.iter().take(3).cloned().collect()
    } else {
        preset_souls.to_vec()
    };

    let mut task_input = String::new();
    let mut cursor: usize = 0;
    let mut help = String::from("Enter:开始  Tab:选Soul  q:退出");

    loop {
        terminal.draw(|f| {
            use ratatui::layout::{Constraint, Direction, Layout, Rect};
            use ratatui::style::{Color, Modifier, Style, Stylize};
            use ratatui::text::{Line, Span, Text};
            use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(3)])
                .split(area);

            // Title
            let title = Paragraph::new("🧠 Soul Agent")
                .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Cyan)));
            f.render_widget(title, chunks[0]);

            // Task input
            let display = if task_input.is_empty() {
                "在此输入你的问题...".to_string()
            } else {
                task_input.clone()
            };
            let input_block = Paragraph::new(Text::from(display))
                .block(Block::default().borders(Borders::ALL).title(" 问题 ").style(Style::default().fg(Color::Yellow)))
                .wrap(Wrap { trim: false });
            f.render_widget(input_block, chunks[1]);

            // Footer: souls + help
            let souls_display = selected.join(", ");
            let footer_text = format!(
                "Souls: {} | Mode: {} | {}",
                souls_display, mode, help
            );
            let footer = Paragraph::new(Text::from(footer_text))
                .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::DarkGray)));
            f.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        disable_raw_mode()?;
                        terminal.backend_mut().execute(LeaveAlternateScreen)?;
                        return Ok((String::new(), vec![]));
                    }
                    KeyCode::Enter => {
                        if !task_input.is_empty() {
                            break;
                        }
                    }
                    KeyCode::Tab => {
                        // Rotate souls selection
                        if available.len() > 3 {
                            let start = ((selected.len() / 3) * 3) % available.len();
                            selected = available.iter().skip(start).take(3).cloned().collect();
                            if selected.len() < 3 {
                                selected.extend(available.iter().take(3 - selected.len()).cloned());
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        task_input.push(c);
                    }
                    KeyCode::Backspace => {
                        task_input.pop();
                    }
                    KeyCode::Left => {
                        cursor = cursor.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        cursor = (cursor + 1).min(task_input.len());
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok((task_input, selected))
}
