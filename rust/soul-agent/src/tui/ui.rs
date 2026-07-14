use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use super::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Title
    f.render_widget(
        Block::default()
            .title(format!(" {} · {} · {} ", truncate(&app.task, 50), app.mode, app.souls.join("、")))
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
        Rect { y: area.y, height: 3, ..area },
    );

    // Tabs
    let all = Tab::all();
    let labels: Vec<String> = all.iter().map(|t| format!(" {} ", t.label())).collect();
    let idx = all.iter().position(|t| *t == app.active_tab).unwrap_or(0);
    f.render_widget(
        Tabs::new(labels)
            .select(idx)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Rect { y: area.y + 3, height: 1, ..area },
    );

    // Content
    let content_y = area.y + 4;
    let footer_h: u16 = if app.follow_up_active { 3 } else { 1 };
    let content_h = area.height.saturating_sub(4 + footer_h).max(1);
    let content = Rect { y: content_y, height: content_h, ..area };

    match app.active_tab {
        Tab::Rounds => draw_rounds(f, app, content),
        Tab::Summary => draw_summary(f, app, content),
        Tab::Info => draw_info(f, app, content),
    }

    // Footer
    if app.follow_up_active {
        let footer = Rect { y: content_y + content_h, height: 3, ..area };
        let display = if app.follow_up_input.is_empty() {
            "追问... (Enter 发送, Esc 取消)".to_string()
        } else {
            format!("> {}█", app.follow_up_input)
        };
        f.render_widget(
            Paragraph::new(Text::from(display))
                .block(Block::default().borders(Borders::ALL).title(" 追问 ").style(Style::default().fg(Color::Yellow))),
            footer,
        );
    } else {
        let footer = Rect { y: content_y + content_h, height: 1, ..area };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
                Span::raw("退出  "),
                Span::styled(" Tab ", Style::default().bg(Color::DarkGray).fg(Color::White)),
                Span::raw("切换  "),
                Span::styled(" ↑↓ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
                Span::raw("滚动  "),
                Span::styled(" / ", Style::default().bg(Color::DarkGray).fg(Color::White)),
                Span::raw("追问  "),
            ])).centered(),
            footer,
        );
    }
}

fn draw_rounds(f: &mut Frame, app: &App, area: Rect) {
    let souls = app.round_souls_with_collisions();
    let mut lines: Vec<Line> = Vec::new();

    if souls.is_empty() {
        lines.push(Line::from(Span::styled("等待 Soul 回应...", Style::default().fg(Color::Gray))));
    } else {
        for (name, content) in &souls {
            lines.push(Line::from(Span::styled(
                format!("── {} ──", name),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for line in content.lines() {
                if line.contains("⚡") {
                    // Collision marker — highlight it
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Red),
                    )));
                } else if line.starts_with("── 收到的") {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    )));
                } else {
                    for chunk in line.as_bytes().chunks(120) {
                        lines.push(Line::from(Span::raw(String::from_utf8_lossy(chunk).to_string())));
                    }
                }
            }
            lines.push(Line::from(""));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(" 交锋过程 "))
            .scroll((app.scroll, 0)),
        area,
    );
}

fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    let text = app.synthesis_text();
    let display = if text.is_empty() { "等待综合摘要...".into() } else { text };
    f.render_widget(
        Paragraph::new(Text::from(display))
            .block(Block::default().borders(Borders::ALL).title(" 综合摘要 "))
            .wrap(Wrap { trim: false })
            .scroll((app.scroll, 0)),
        area,
    );
}

fn draw_info(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("参与 Soul: {} 个", app.souls.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for (i, name) in app.souls.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!("{}. {}", i + 1, name),
            Style::default().fg(Color::Green),
        )));
    }

    // Cost info
    let costs: Vec<&str> = app.events.iter()
        .filter(|e| matches!(e.event_type, possession::WsEventType::Cost))
        .map(|e| e.payload.as_str())
        .collect();
    if !costs.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("成本:", Style::default().add_modifier(Modifier::BOLD))));
        for c in &costs {
            lines.push(Line::from(Span::raw(*c)));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(" 信息 "))
            .scroll((app.scroll, 0)),
        area,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
