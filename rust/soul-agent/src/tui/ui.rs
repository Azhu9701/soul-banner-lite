use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use super::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Title bar ──
    f.render_widget(
        Block::default()
            .title(format!(" Soul Agent · {} · {} mode · {} souls ", truncate(&app.task, 40), app.mode, app.souls.len()))
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
        Rect { y: area.y, height: 3, ..area },
    );

    // ── Tab bar ──
    let all_tabs = Tab::all();
    let tab_labels: Vec<String> = all_tabs.iter().map(|t| {
        let extra = match t {
            Tab::Rounds => format!(" (R{}/{})", app.current_round, app.total_rounds),
            Tab::Collisions => format!(" ({})", app.collision_count),
            Tab::Souls => format!(" ({})", app.souls.len()),
            _ => String::new(),
        };
        format!(" {} ", t.label()) + &extra
    }).collect();

    let tab_idx = all_tabs.iter().position(|t| *t == app.active_tab).unwrap_or(0);
    f.render_widget(
        Tabs::new(tab_labels)
            .select(tab_idx)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Rect { y: area.y + 3, height: 1, ..area },
    );

    // ── Content ──
    let content_y = area.y + 4;
    let footer_h: u16 = if app.follow_up_active { 3 } else { 1 };
    let content_height = area.height.saturating_sub(4 + footer_h);
    let content_area = Rect { y: content_y, height: content_height.max(1), ..area };

    match app.active_tab {
        Tab::Verdict => draw_verdict_structured(f, app, content_area),
        Tab::Rounds => draw_rounds(f, app, content_area),
        Tab::Collisions => draw_tab_list(f, app, content_area, " 碰撞检测 ", &app.collisions(), "⚡"),
        Tab::Souls => draw_souls(f, app, content_area),
        Tab::Cost => draw_tab_list(f, app, content_area, " 成本 ", &app.cost_info(), "💰"),
    }

    // ── Footer ──
    if app.follow_up_active {
        let footer_rect = Rect { y: content_y + content_height, height: 3, ..area };
        let display = if app.follow_up_input.is_empty() {
            "输入追问... (Enter 发送, Esc 取消)".to_string()
        } else {
            format!("> {}█", app.follow_up_input)
        };
        f.render_widget(
            Paragraph::new(Text::from(display))
                .block(Block::default().borders(Borders::ALL).title(" 追问 ").style(Style::default().fg(Color::Yellow))),
            footer_rect,
        );
    } else {
        let footer_rect = Rect { y: content_y + content_height, height: 1, ..area };
        let footer = Line::from(vec![
            Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw("退出  "),
            Span::styled(" Tab ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw("切换面板  "),
            Span::styled(" ↑↓ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw("滚动  "),
            Span::styled(" ←→ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw("轮次  "),
            Span::styled(" / ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw("追问  "),
        ]);
        f.render_widget(Paragraph::new(footer).centered(), footer_rect);
    }
}

fn draw_tab(f: &mut Frame, app: &App, area: Rect, title: &str, text: &str) {
    f.render_widget(
        Paragraph::new(Text::from(text.to_string()))
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: false })
            .scroll((app.scroll, 0)),
        area,
    );
}

fn draw_tab_list(f: &mut Frame, app: &App, area: Rect, title: &str, items: &[String], _prefix: &str) {
    let lines: Vec<Line> = if items.is_empty() {
        vec![Line::from(Span::styled("暂无数据", Style::default().fg(Color::Gray)))]
    } else {
        items.iter().map(|s| Line::from(Span::raw(s.as_str()))).collect()
    };
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(title))
            .scroll((app.scroll, 0)),
        area,
    );
}

fn draw_rounds(f: &mut Frame, app: &App, area: Rect) {
    let souls = app.round_souls();
    let mut lines: Vec<Line> = Vec::new();

    if souls.is_empty() {
        lines.push(Line::from(Span::styled("暂无 Soul 输出数据", Style::default().fg(Color::Gray))));
    } else {
        for (name, content) in &souls {
            lines.push(Line::from(Span::styled(
                format!("── {} ──", name),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )));
            for line in content.lines() {
                for chunk in line.as_bytes().chunks(120) {
                    lines.push(Line::from(Span::raw(String::from_utf8_lossy(chunk).to_string())));
                }
            }
            lines.push(Line::from(""));
        }
    }

    let title = format!(" 交锋过程 (R{}/{}) ", app.current_round, app.total_rounds);
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(title))
            .scroll((app.scroll, 0)),
        area,
    );
}

fn draw_souls(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("参与 Soul: {} 个", app.souls.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    for (i, name) in app.souls.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!("{}. {}", i + 1, name),
            Style::default().fg(Color::Green),
        )));
    }
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().borders(Borders::ALL).title(" Soul 信息 "))
            .scroll((app.scroll, 0)),
        area,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

/// Render structured decision report, falling back to raw text
fn draw_verdict_structured(f: &mut Frame, app: &App, area: Rect) {
    if let Some(s) = app.structured_synthesis() {
        let mut lines: Vec<Line> = Vec::new();

        // Consensus
        if !s.consensus.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("✅ 共识 ({})", s.consensus.len()),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            for (i, c) in s.consensus.iter().enumerate() {
                lines.push(Line::from(Span::raw(format!("{}. {}", i + 1, c.point))));
                lines.push(Line::from(Span::styled(
                    format!("   共享者: {} | {}", c.shared_by.join(", "), c.depth),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            lines.push(Line::from(""));
        }

        // Divergence
        if !s.divergence.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("❌ 分歧 ({})", s.divergence.len()),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            for (i, d) in s.divergence.iter().enumerate() {
                lines.push(Line::from(Span::raw(format!("{}. {} [{}]", i + 1, d.axis, d.divergence_type))));
                for pos in &d.positions {
                    lines.push(Line::from(Span::styled(
                        format!("   {}: {}", pos.soul_name, pos.stance),
                        Style::default().fg(Color::Yellow),
                    )));
                }
            }
            lines.push(Line::from(""));
        }

        // Blind spots
        if !s.blind_spots.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("🔍 盲点 ({})", s.blind_spots.len()),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )));
            for (i, b) in s.blind_spots.iter().enumerate() {
                let structural = if b.is_structural { " [结构性]" } else { "" };
                lines.push(Line::from(Span::raw(format!("{}. {} — {}{}", i + 1, b.dimension, b.missing_perspective, structural))));
                if let Some(ref suggested) = b.suggested_soul {
                    lines.push(Line::from(Span::styled(
                        format!("   建议补充: {}", suggested),
                        Style::default().fg(Color::Cyan),
                    )));
                }
            }
            lines.push(Line::from(""));
        }

        // Principal contradiction
        lines.push(Line::from(Span::styled(
            format!("⚡ 核心矛盾: {}", s.principal_contradiction.description),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::raw(format!("   涉及方: {}", s.principal_contradiction.parties.join(", ")))));
        lines.push(Line::from(""));

        // Action program
        if !s.action_program.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("📋 行动建议 ({})", s.action_program.len()),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            for a in &s.action_program {
                let priority = match a.priority {
                    1 => "P1".to_string(),
                    2 => "P2".to_string(),
                    _ => "P3".to_string(),
                };
                lines.push(Line::from(Span::styled(
                    format!("  {} [{}] {}: {}", priority, a.timeline, a.direction, a.rationale),
                    Style::default().fg(Color::White),
                )));
            }
        }

        let block = Block::default().borders(Borders::ALL).title(format!(
            " 决策报告 | 共识:{} 分歧:{} 盲点:{} ",
            s.consensus.len(), s.divergence.len(), s.blind_spots.len()
        ));
        f.render_widget(
            Paragraph::new(Text::from(lines)).block(block).scroll((app.scroll, 0)),
            area,
        );
    } else {
        // Fallback to raw text
        draw_tab(f, app, area, " 综合裁决 ", &app.synthesis_text());
    }
}
