use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use super::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // ── Title bar ──
    let title = format!(
        " Soul Agent · {} · {} mode · {} souls ",
        truncate(&app.task, 40),
        app.mode,
        app.souls.len()
    );
    let title_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let inner = title_block.inner(area);
    f.render_widget(title_block, area);

    // ── Tab bar ──
    let tabs = [
        format!(" 最终裁决 "),
        format!(" 交锋 (R{}/{}) ", app.current_round, app.total_rounds),
        format!(" 碰撞:{} ", app.collision_count),
        format!(" Souls:{} ", app.souls.len()),
        format!(" 成本 "),
    ];
    let tab_idx = match app.active_tab {
        Tab::Verdict => 0,
        Tab::Rounds => 1,
        Tab::Collisions => 2,
        Tab::Souls => 3,
        Tab::Cost => 4,
    };

    let tab_widget = Tabs::new(tabs)
        .select(tab_idx)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let tab_area = Rect { y: inner.y, height: 2, ..inner };
    f.render_widget(tab_widget, tab_area);

    // ── Content area ──
    let content_area = Rect {
        y: inner.y + 2,
        height: inner.height.saturating_sub(4),
        ..inner
    };

    match app.active_tab {
        Tab::Verdict => draw_verdict(f, app, content_area),
        Tab::Rounds => draw_rounds(f, app, content_area),
        Tab::Collisions => draw_collisions(f, app, content_area),
        Tab::Souls => draw_souls(f, app, content_area),
        Tab::Cost => draw_cost(f, app, content_area),
    }

    // ── Footer ──
    let footer = Line::from(vec![
        Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" 退出  "),
        Span::styled(" Tab ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" 切换面板  "),
        Span::styled(" ↑↓ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" 滚动  "),
        Span::styled(" ←→ ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" 切换轮次  "),
    ]);
    let footer_area = Rect {
        y: inner.height.saturating_sub(1),
        height: 1,
        ..inner
    };
    f.render_widget(Paragraph::new(footer).centered(), footer_area);
}

// ── Tab 1: 最终裁决 ──

fn draw_verdict(f: &mut Frame, app: &App, area: Rect) {
    let text = app.synthesis_text();
    let paragraph = Paragraph::new(Text::from(text))
        .block(Block::default().borders(Borders::ALL).title(" 综合裁决 "))
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

// ── Tab 2: 交锋过程 ──

fn draw_rounds(f: &mut Frame, app: &App, area: Rect) {
    let souls = app.round_souls();
    let mut lines: Vec<Line> = Vec::new();

    for (name, content) in &souls {
        lines.push(Line::from(Span::styled(
            format!("── {} ──", name),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for line in content.lines().take(20) {
            lines.push(Line::from(Span::raw(line)));
        }
        lines.push(Line::from(""));
    }

    if lines.is_empty() {
        lines.push(Line::from("等待 Soul 回应..."));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(format!(" Round {}/{} ", app.current_round, app.total_rounds)))
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

// ── Tab 3: 碰撞图谱 ──

fn draw_collisions(f: &mut Frame, app: &App, area: Rect) {
    let collisions = app.collisions();
    let mut lines: Vec<Line> = Vec::new();

    if collisions.is_empty() {
        lines.push(Line::from(Span::styled(
            "本轮未检测到碰撞",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for (i, c) in collisions.iter().enumerate() {
            lines.push(Line::from(Span::styled(
                format!("{} ⚡ {}", i + 1, c),
                Style::default().fg(Color::Red),
            )));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(" 碰撞检测 "))
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

// ── Tab 4: Soul 信息 ──

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

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(" Soul 信息 "))
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

// ── Tab 5: 成本 ──

fn draw_cost(f: &mut Frame, app: &App, area: Rect) {
    let costs = app.cost_info();
    let mut lines: Vec<Line> = Vec::new();

    if costs.is_empty() {
        lines.push(Line::from(Span::styled(
            "等待成本统计...",
            Style::default().fg(Color::Gray),
        )));
    } else {
        for c in &costs {
            lines.push(Line::from(Span::raw(c)));
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(" 成本 "))
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
