use possession::WsEvent;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Rounds,
    Summary,
    Info,
}

impl Tab {
    pub fn label(&self) -> &str {
        match self {
            Tab::Rounds => "交锋过程",
            Tab::Summary => "摘要",
            Tab::Info => "信息",
        }
    }

    pub fn all() -> [Tab; 3] {
        [Tab::Rounds, Tab::Summary, Tab::Info]
    }
}

pub struct App {
    pub task: String,
    pub souls: Vec<String>,
    pub mode: String,
    pub events: Vec<WsEvent>,
    pub active_tab: Tab,
    pub scroll: u16,
    pub follow_up_input: String,
    pub follow_up_active: bool,
}

impl App {
    pub fn new(task: String, souls: Vec<String>, mode: String, events: Vec<WsEvent>) -> Self {
        App {
            task, souls, mode, events,
            active_tab: Tab::Rounds,
            scroll: 0,
            follow_up_input: String::new(),
            follow_up_active: false,
        }
    }

    /// Get each soul's full text, with collision markers embedded inline
    /// Returns (soul_name, text_with_collision_markers)
    pub fn round_souls_with_collisions(&self) -> Vec<(String, String)> {
        // First pass: build a map of (from_soul, to_soul) -> collision content
        let mut collision_map: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
        for e in &self.events {
            if matches!(e.event_type, possession::WsEventType::Collision) {
                // Try to parse collision from payload
                let parts: Vec<&str> = e.payload.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let from = parts[0].to_string();
                    let to = parts.get(1).map(|s| s.trim().trim_end_matches(':').to_string()).unwrap_or_default();
                    let content = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
                    collision_map.insert((from, to), content);
                }
            }
        }

        // Second pass: collect soul texts with collision markers
        let mut souls: Vec<(String, String)> = Vec::new();
        let mut current_name = String::new();
        let mut current_text = String::new();

        for e in &self.events {
            match e.event_type {
                possession::WsEventType::SoulStarted => {
                    if !current_name.is_empty() && !current_text.is_empty() {
                        // Inject collisions targeting this soul
                        let with_collisions = Self::inject_collisions(&current_name, &current_text, &collision_map);
                        souls.push((std::mem::take(&mut current_name), with_collisions));
                        current_text.clear();
                    }
                    current_name = e.soul_name.clone().unwrap_or_default();
                }
                possession::WsEventType::SoulChunk => {
                    current_text.push_str(&e.payload);
                }
                possession::WsEventType::SoulDone => {
                    if !current_text.is_empty() {
                        let with_collisions = Self::inject_collisions(&current_name, &current_text, &collision_map);
                        souls.push((std::mem::take(&mut current_name), with_collisions));
                        current_text.clear();
                    }
                }
                _ => {}
            }
        }

        if !current_name.is_empty() && !current_text.is_empty() {
            let with_collisions = Self::inject_collisions(&current_name, &current_text, &collision_map);
            souls.push((current_name, with_collisions));
        }

        souls
    }

    fn inject_collisions(
        soul_name: &str,
        text: &str,
        collision_map: &std::collections::HashMap<(String, String), String>,
    ) -> String {
        let mut result = text.to_string();
        let targets: Vec<String> = collision_map.iter()
            .filter(|((_, to), _)| to == soul_name)
            .map(|((from, _), content)| format!("\n  ⚡ {} 说道: {}\n", from, content))
            .collect();
        if !targets.is_empty() {
            result.push_str("\n\n── 收到的交叉质证 ──\n");
            for t in &targets {
                result.push_str(t);
            }
        }
        result
    }

    /// Get raw soul texts without collision injection
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
                possession::WsEventType::SoulChunk => { current_text.push_str(&e.payload); }
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

    pub fn synthesis_text(&self) -> String {
        let mut text = String::new();
        for e in &self.events {
            if matches!(e.event_type, possession::WsEventType::SynthesisChunk) {
                text.push_str(&e.payload);
            }
        }
        text
    }

    // ── Navigation ──

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

    pub fn toggle_follow_up(&mut self) {
        self.follow_up_active = !self.follow_up_active;
        if !self.follow_up_active { self.follow_up_input.clear(); }
    }
    pub fn follow_up_push(&mut self, c: char) { self.follow_up_input.push(c); }
    pub fn follow_up_pop(&mut self) { self.follow_up_input.pop(); }
}
