use possession::WsEvent;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Verdict,
    Rounds,
    Collisions,
    Souls,
    Cost,
}

pub struct App {
    pub task: String,
    pub souls: Vec<String>,
    pub mode: String,
    pub events: Vec<WsEvent>,
    pub active_tab: Tab,
    pub scroll: u16,
    pub current_round: usize,
    pub total_rounds: usize,
    pub collision_count: usize,
    #[allow(dead_code)]
    pub total_tokens: u64,
}

impl App {
    pub fn new(task: String, souls: Vec<String>, mode: String, events: Vec<WsEvent>) -> Self {
        let (total_rounds, collision_count, total_tokens) = Self::analyze_events(&events);

        App {
            task,
            souls,
            mode,
            events,
            active_tab: Tab::Verdict,
            scroll: 0,
            current_round: 1,
            total_rounds,
            collision_count,
            total_tokens,
        }
    }

    fn analyze_events(events: &[WsEvent]) -> (usize, usize, u64) {
        let process_steps = events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::ProcessStep))
            .count();
        let rounds = (process_steps / 3).min(1) + 1; // rough estimate

        let collisions = events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Collision))
            .count();

        let _tokens: u64 = 0; // TODO: parse from Cost events

        (rounds, collisions, 0u64)
    }

    /// 获取综合裁决文本
    pub fn synthesis_text(&self) -> String {
        let mut text = String::new();
        for e in &self.events {
            if matches!(e.event_type, possession::WsEventType::SynthesisChunk) {
                text.push_str(&e.payload);
            }
        }
        if text.is_empty() { "等待综合裁决...".into() } else { text }
    }

    /// 获取指定轮次的 Soul 输出
    pub fn round_souls(&self) -> Vec<(String, String)> {
        let mut souls: Vec<(String, String)> = Vec::new();
        let mut current_name = String::new();
        let mut current_text = String::new();

        for e in &self.events {
            match e.event_type {
                possession::WsEventType::SoulStarted => {
                    if !current_name.is_empty() && !current_text.is_empty() {
                        souls.push((current_name.clone(), current_text.clone()));
                        current_text.clear();
                    }
                    current_name = e.soul_name.clone().unwrap_or_default();
                }
                possession::WsEventType::SoulChunk => {
                    current_text.push_str(&e.payload);
                }
                possession::WsEventType::SoulDone => {
                    if !current_text.is_empty() {
                        souls.push((current_name.clone(), current_text.clone()));
                        current_text.clear();
                        current_name.clear();
                    }
                }
                _ => {}
            }
        }

        if !current_name.is_empty() && !current_text.is_empty() {
            souls.push((current_name, current_text));
        }

        souls
    }

    /// 获取碰撞列表
    pub fn collisions(&self) -> Vec<String> {
        self.events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Collision))
            .map(|e| e.payload.clone())
            .collect()
    }

    /// 获取成本信息
    pub fn cost_info(&self) -> Vec<String> {
        self.events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Cost))
            .map(|e| e.payload.clone())
            .collect()
    }

    // ── Navigation ──

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Verdict => Tab::Rounds,
            Tab::Rounds => Tab::Collisions,
            Tab::Collisions => Tab::Souls,
            Tab::Souls => Tab::Cost,
            Tab::Cost => Tab::Verdict,
        };
        self.scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            Tab::Verdict => Tab::Cost,
            Tab::Rounds => Tab::Verdict,
            Tab::Collisions => Tab::Rounds,
            Tab::Souls => Tab::Collisions,
            Tab::Cost => Tab::Souls,
        };
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self) { self.scroll = self.scroll.saturating_sub(1); }
    pub fn scroll_down(&mut self) { self.scroll = self.scroll.saturating_add(1); }
    pub fn next_round(&mut self) { if self.current_round < self.total_rounds { self.current_round += 1; } }
    pub fn prev_round(&mut self) { self.current_round = self.current_round.saturating_sub(1).max(1); }
}
