use possession::WsEvent;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Verdict,
    Rounds,
    Collisions,
    Souls,
    Cost,
}

impl Tab {
    pub fn label(&self) -> &str {
        match self {
            Tab::Verdict => "最终裁决",
            Tab::Rounds => "交锋过程",
            Tab::Collisions => "碰撞图谱",
            Tab::Souls => "Soul 信息",
            Tab::Cost => "成本",
        }
    }

    pub fn all() -> [Tab; 5] {
        [Tab::Verdict, Tab::Rounds, Tab::Collisions, Tab::Souls, Tab::Cost]
    }
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
    /// 追问输入模式
    pub follow_up_input: String,
    pub follow_up_active: bool,
}

impl App {
    pub fn new(task: String, souls: Vec<String>, mode: String, events: Vec<WsEvent>) -> Self {
        let (total_rounds, collision_count) = Self::analyze_events(&events);

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
            total_tokens: 0,
            follow_up_input: String::new(),
            follow_up_active: false,
        }
    }

    fn analyze_events(events: &[WsEvent]) -> (usize, usize) {
        let process_steps = events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::ProcessStep))
            .count();
        let rounds = (process_steps / 3).min(1) + 1;

        let collisions = events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Collision))
            .count();

        (rounds, collisions)
    }

    pub fn synthesis_text(&self) -> String {
        let mut text = String::new();
        for e in &self.events {
            if matches!(e.event_type, possession::WsEventType::SynthesisChunk) {
                text.push_str(&e.payload);
            }
        }
        if text.is_empty() { "等待综合裁决...".into() } else { text }
    }

    pub fn round_souls(&self) -> Vec<(String, String)> {
        let mut souls: Vec<(String, String)> = Vec::new();
        let mut current_name = String::new();
        let mut current_text = String::new();

        for e in &self.events {
            match e.event_type {
                possession::WsEventType::SoulStarted => {
                    if !current_name.is_empty() && !current_text.is_empty() {
                        souls.push((std::mem::take(&mut current_name), std::mem::take(&mut current_text)));
                    }
                    current_name = e.soul_name.clone().unwrap_or_default();
                }
                possession::WsEventType::SoulChunk => {
                    current_text.push_str(&e.payload);
                }
                possession::WsEventType::SoulDone => {
                    if !current_text.is_empty() {
                        souls.push((std::mem::take(&mut current_name), std::mem::take(&mut current_text)));
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

    pub fn collisions(&self) -> Vec<String> {
        self.events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Collision))
            .map(|e| e.payload.clone())
            .collect()
    }

    pub fn cost_info(&self) -> Vec<String> {
        self.events.iter()
            .filter(|e| matches!(e.event_type, possession::WsEventType::Cost))
            .map(|e| e.payload.clone())
            .collect()
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + 1) % tabs.len()];
        self.scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.active_tab).unwrap_or(0);
        self.active_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self) { self.scroll = self.scroll.saturating_sub(1); }
    pub fn scroll_down(&mut self) { self.scroll = self.scroll.saturating_add(1); }
    pub fn next_round(&mut self) { if self.current_round < self.total_rounds { self.current_round += 1; } }
    pub fn prev_round(&mut self) { self.current_round = self.current_round.saturating_sub(1).max(1); }

    /// 切换追问输入模式
    pub fn toggle_follow_up(&mut self) {
        self.follow_up_active = !self.follow_up_active;
        if !self.follow_up_active {
            self.follow_up_input.clear();
        }
    }

    /// 输入字符到追问框
    pub fn follow_up_push(&mut self, c: char) {
        self.follow_up_input.push(c);
    }

    pub fn follow_up_pop(&mut self) {
        self.follow_up_input.pop();
    }
}
