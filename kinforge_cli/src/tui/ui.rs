use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

use super::state::{InputMode, Tab, TaskRow, TuiState};
use kinforge_core::models::{TaskPriority, TaskStatus};

pub fn draw(frame: &mut Frame, state: &TuiState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_tab_bar(frame, state, chunks[0]);

    match state.active_tab {
        Tab::People => draw_people(frame, state, chunks[1]),
        Tab::Tasks => draw_tasks(frame, state, chunks[1]),
        Tab::Stats => draw_stats(frame, state, chunks[1]),
    }

    draw_status(frame, state, chunks[2]);
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

fn draw_tab_bar(frame: &mut Frame, state: &TuiState, area: Rect) {
    let labels = vec![
        Line::from(format!(
            "  People ({})  ",
            state.filtered_people.len()
        )),
        Line::from(format!("  Tasks ({})  ", state.tasks.len())),
        Line::from("  Stats  "),
    ];

    let tabs = Tabs::new(labels)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Kinforge Family History "),
        )
        .select(state.active_tab.index())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("|", Style::default().fg(Color::DarkGray)));

    frame.render_widget(tabs, area);
}

// ── People tab ────────────────────────────────────────────────────────────────

fn draw_people(frame: &mut Frame, state: &TuiState, area: Rect) {
    if state.detail_open {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(area);
        draw_people_list(frame, state, chunks[0]);
        draw_person_detail(frame, state, chunks[1]);
    } else {
        draw_people_list(frame, state, area);
    }
}

fn draw_people_list(frame: &mut Frame, state: &TuiState, area: Rect) {
    let title = if state.search_active {
        format!(
            " People — Filter: {} ({} match) ",
            state.search_query,
            state.filtered_people.len()
        )
    } else if !state.search_query.is_empty() {
        format!(
            " People — \"{}\" ({}/{}) ",
            state.search_query,
            state.filtered_people.len(),
            state.people.len()
        )
    } else {
        format!(" People ({}) ", state.people.len())
    };

    let items: Vec<ListItem> = state
        .filtered_people
        .iter()
        .map(|&idx| {
            let row = &state.people[idx];
            let year = row
                .birth_year
                .map(|y| format!("b.{}", y))
                .unwrap_or_default();
            let line = Line::from(vec![
                Span::raw(truncate(&row.display_name, 28)),
                Span::styled(
                    format!("  {}", year),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.filtered_people.is_empty() {
        list_state.select(Some(state.people_selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_person_detail(frame: &mut Frame, state: &TuiState, area: Rect) {
    let name = state
        .detail_person_id
        .as_ref()
        .and_then(|pid| state.people.iter().find(|p| &p.id == pid))
        .map(|p| p.display_name.as_str())
        .unwrap_or("—");

    let mut lines: Vec<Line> = Vec::new();

    // Events section
    lines.push(Line::from(Span::styled(
        " Events",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    if state.detail_events.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no events recorded)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for event in &state.detail_events {
            let date_str = event
                .date
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".to_string());
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<14}", truncate(&event.event_type.to_string(), 14)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(date_str),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Relationships section
    lines.push(Line::from(Span::styled(
        " Relationships",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ─────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    if state.detail_rel_rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no relationships recorded)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (label, other_name) in &state.detail_rel_rows {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<18}", truncate(label, 18)),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(other_name.as_str()),
            ]));
        }
    }

    let scroll = state.detail_scroll as u16;

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", name)),
        )
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

// ── Tasks tab ─────────────────────────────────────────────────────────────────

fn draw_tasks(frame: &mut Frame, state: &TuiState, area: Rect) {
    let items: Vec<ListItem> = state
        .task_rows
        .iter()
        .map(|row| match row {
            TaskRow::Header(h) => ListItem::new(Line::from(Span::styled(
                format!(" ── {} ", h),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ))),
            TaskRow::Item(task_idx) => {
                let task = &state.tasks[*task_idx];

                let status_span = match task.status {
                    TaskStatus::Pending => {
                        Span::styled("[ ] ", Style::default().fg(Color::DarkGray))
                    }
                    TaskStatus::InProgress => {
                        Span::styled("[~] ", Style::default().fg(Color::Yellow))
                    }
                    TaskStatus::Done => {
                        Span::styled("[✓] ", Style::default().fg(Color::Green))
                    }
                };

                let prio_span = match task.priority {
                    TaskPriority::High => Span::styled(
                        "HIGH ",
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD),
                    ),
                    TaskPriority::Medium => {
                        Span::styled("MED  ", Style::default().fg(Color::Yellow))
                    }
                    TaskPriority::Low => {
                        Span::styled("LOW  ", Style::default().fg(Color::DarkGray))
                    }
                };

                let desc = Span::raw(truncate(&task.description, 58));
                ListItem::new(Line::from(vec![status_span, prio_span, desc]))
            }
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.task_rows.is_empty() {
        list_state.select(Some(state.tasks_selected));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Research Tasks ({}) ", state.tasks.len())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

// ── Stats tab ─────────────────────────────────────────────────────────────────

fn draw_stats(frame: &mut Frame, state: &TuiState, area: Rect) {
    let mut lines: Vec<Line> = vec![Line::from("")];

    match &state.stats {
        Some(s) => {
            let rows: [(&str, u64); 6] = [
                ("People", s.people),
                ("Events", s.events),
                ("Relationships", s.relationships),
                ("Places", s.places),
                ("Sources", s.sources),
                ("Citations", s.citations),
            ];
            for (label, count) in &rows {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<18}", label),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(
                        count.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
        None => {
            lines.push(Line::from(Span::styled(
                "  (statistics unavailable)",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "    Database          ",
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(&state.db_path, Style::default().fg(Color::DarkGray)),
    ]));

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Database Statistics "),
        );

    frame.render_widget(para, area);
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn draw_status(frame: &mut Frame, state: &TuiState, area: Rect) {
    let text = match state.mode {
        InputMode::Search => format!(
            " ESC: cancel  Backspace: delete  [{} match(es)]",
            state.filtered_people.len()
        ),
        InputMode::Detail => " ESC: close panel  ↑↓/jk: scroll  q: quit".to_string(),
        InputMode::Normal => match state.active_tab {
            Tab::People => {
                " Tab: next tab  ↑↓/jk: navigate  /: search  Enter: detail  q: quit"
                    .to_string()
            }
            Tab::Tasks => " Tab: next tab  ↑↓/jk: navigate  q: quit".to_string(),
            Tab::Stats => " Tab: next tab  q: quit".to_string(),
        },
    };

    let para = Paragraph::new(Line::from(Span::styled(
        text,
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(para, area);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn truncate(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}
