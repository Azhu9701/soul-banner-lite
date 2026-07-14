//! Soul Agent TUI — 质量优先的多面板终端界面

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
    let gateway = Arc::new(GatewayRegistry::new());
    let engine = PossessionEngine::new(store, registry, gateway, config.domain);

    let mode_enum = match mode.as_str() {
        "single" => Some(PossessionMode::Single),
        "debate" => Some(PossessionMode::Debate),
        _ => Some(PossessionMode::Conference),
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsEvent>(256);
    let input = PossessionInput {
        task: task.clone(),
        souls: souls.clone(),
        mode: mode_enum,
        topic: None,
        judgment: None,
        worry: None,
        unknown: None,
        interrogation_context: None,
        search_topic: false,
        search_results: None,
        task_cards: Default::default(),
    };

    engine.start_possession(input, tx).await?;

    let mut events: Vec<WsEvent> = Vec::new();
    while let Some(event) = rx.recv().await {
        events.push(event);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(task, souls, mode, events);

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
