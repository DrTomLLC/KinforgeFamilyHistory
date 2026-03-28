use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame,
};

use super::state::{InputMode, Tab, TaskRow, TaskStatusFilter, TuiState, TUI_EVENT_TYPES, TUI_REL_TYPES};
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
        Tab::Sources => draw_sources(frame, state, chunks[1]),
        Tab::Stats => draw_stats(frame, state, chunks[1]),
    }

    draw_status(frame, state, chunks[2]);
}

// ── Tab bar ───────────────────────────────────────────────────────────────────

fn draw_tab_bar(frame: &mut Frame, state: &TuiState, area: Rect) {
    let labels = vec![
        Line::from(format!("  People ({})  ", state.filtered_people.len())),
        Line::from(format!("  Tasks ({})  ", state.tasks.len())),
        Line::from(format!("  Sources ({})  ", state.sources.len())),
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

    if state.mode == InputMode::PersonCreate {
        draw_person_create_popup(frame, state, area);
    }
    if state.mode == InputMode::PersonEdit {
        draw_person_edit_popup(frame, state, area);
    }
    if state.mode == InputMode::ConfirmDelete {
        draw_confirm_delete_popup(frame, state, area);
    }
    if state.mode == InputMode::EventCreate {
        draw_event_create_popup(frame, state, area);
    }
    if state.mode == InputMode::RelationshipCreate {
        draw_rel_create_popup(frame, state, area);
    }
    if state.mode == InputMode::PersonNotesEdit {
        draw_person_notes_popup(frame, state, area);
    }
    if state.mode == InputMode::EventEdit {
        draw_event_edit_popup(frame, state, area);
    }
}

fn draw_confirm_delete_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 52_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 1;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 3 };

    let name = truncate(&state.confirm_name, 30);
    let msg = format!("  Delete \"{}\"?  y = confirm · any key = cancel", name);

    let para = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(Color::White),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Delete ")
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_person_edit_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 50_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 2;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 5 };

    let cursor = "_";
    let given_text = if state.person_edit_field == 0 {
        format!("{}{}", state.person_edit_given, cursor)
    } else {
        state.person_edit_given.clone()
    };
    let surname_text = if state.person_edit_field == 1 {
        format!("{}{}", state.person_edit_surname, cursor)
    } else {
        state.person_edit_surname.clone()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Given:   ", Style::default().fg(Color::Cyan)),
            Span::styled(given_text, if state.person_edit_field == 0 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Surname: ", Style::default().fg(Color::Cyan)),
            Span::styled(surname_text, if state.person_edit_field == 1 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit Name — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_person_create_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 50_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 3;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: 7,
    };

    let cursor = "_";
    let given_text = if state.person_create_field == 0 {
        format!("{}{}", state.person_create_given, cursor)
    } else {
        state.person_create_given.clone()
    };
    let surname_text = if state.person_create_field == 1 {
        format!("{}{}", state.person_create_surname, cursor)
    } else {
        state.person_create_surname.clone()
    };
    let sex_labels = ["Unknown", "Male", "Female"];
    let sex_text = if state.person_create_field == 2 {
        format!("◄ {} ►", sex_labels[state.person_create_sex as usize])
    } else {
        sex_labels[state.person_create_sex as usize].to_string()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Given:   ", Style::default().fg(Color::Cyan)),
            Span::styled(given_text, if state.person_create_field == 0 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Surname: ", Style::default().fg(Color::Cyan)),
            Span::styled(surname_text, if state.person_create_field == 1 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Sex:     ", Style::default().fg(Color::Cyan)),
            Span::styled(sex_text, if state.person_create_field == 2 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New Person — Tab: switch · ←/→: sex · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Green)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_event_create_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 58_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 3;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 7 };

    let type_name = TUI_EVENT_TYPES[state.event_create_type_idx];
    let type_text = if state.event_create_field == 0 {
        format!("◄ {} ►", type_name)
    } else {
        type_name.to_string()
    };

    let cursor = "_";
    let date_text = if state.event_create_field == 1 {
        format!("{}{}", state.event_create_date, cursor)
    } else {
        state.event_create_date.clone()
    };
    let place_text = if state.event_create_field == 2 {
        format!("{}{}", state.event_create_place, cursor)
    } else {
        state.event_create_place.clone()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Type:   ", Style::default().fg(Color::Cyan)),
            Span::styled(type_text, if state.event_create_field == 0 { active } else { inactive }),
        ]),
        Line::from(Span::styled("          ←/→ cycle types", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Date:   ", Style::default().fg(Color::Cyan)),
            Span::styled(date_text, if state.event_create_field == 1 { active } else { inactive }),
            Span::styled(" (YYYY-MM-DD, optional)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Place:  ", Style::default().fg(Color::Cyan)),
            Span::styled(place_text, if state.event_create_field == 2 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Add Event — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_rel_create_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 60_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 3;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 6 };

    let (rel_label, _) = TUI_REL_TYPES[state.rel_create_type_idx];
    let rel_text = if state.rel_create_field == 1 {
        format!("◄ {} ►", rel_label)
    } else {
        rel_label.to_string()
    };

    let cursor = "_";
    let person2_text = if state.rel_create_field == 0 {
        format!("{}{}", state.rel_create_person2_buf, cursor)
    } else {
        state.rel_create_person2_buf.clone()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Type:    ", Style::default().fg(Color::Cyan)),
            Span::styled(rel_text, if state.rel_create_field == 1 { active } else { inactive }),
        ]),
        Line::from(Span::styled("           Tab: switch  ←/→: cycle type", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Person:  ", Style::default().fg(Color::Cyan)),
            Span::styled(person2_text, if state.rel_create_field == 0 { active } else { inactive }),
        ]),
        Line::from(Span::styled("           (type name fragment — first match used)", Style::default().fg(Color::DarkGray))),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Add Relationship — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Magenta)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_person_notes_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 64_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 1;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 3 };

    let input_display = format!("  {}_", state.person_notes_buf);
    let para = Paragraph::new(Line::from(Span::styled(
        input_display,
        Style::default().fg(Color::White),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit Notes — Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_event_edit_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 58_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 3;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 7 };

    let type_name = TUI_EVENT_TYPES[state.event_edit_type_idx];
    let type_text = if state.event_edit_field == 0 {
        format!("◄ {} ►", type_name)
    } else {
        type_name.to_string()
    };
    let cursor = "_";
    let date_text = if state.event_edit_field == 1 {
        format!("{}{}", state.event_edit_date, cursor)
    } else {
        state.event_edit_date.clone()
    };
    let place_text = if state.event_edit_field == 2 {
        format!("{}{}", state.event_edit_place, cursor)
    } else {
        state.event_edit_place.clone()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Type:   ", Style::default().fg(Color::Cyan)),
            Span::styled(type_text, if state.event_edit_field == 0 { active } else { inactive }),
        ]),
        Line::from(Span::styled("          ←/→ cycle types", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Date:   ", Style::default().fg(Color::Cyan)),
            Span::styled(date_text, if state.event_edit_field == 1 { active } else { inactive }),
            Span::styled(" (YYYY-MM-DD, optional)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Place:  ", Style::default().fg(Color::Cyan)),
            Span::styled(place_text, if state.event_edit_field == 2 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit Event — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_people_list(frame: &mut Frame, state: &TuiState, area: Rect) {
    let sort_badge = format!("[sort: {}]", state.sort_order.label());
    let title = if state.search_active {
        format!(
            " People — Filter: {} ({} match) ",
            state.search_query,
            state.filtered_people.len()
        )
    } else if !state.search_query.is_empty() {
        format!(
            " People — \"{}\" ({}/{}) {} ",
            state.search_query,
            state.filtered_people.len(),
            state.people.len(),
            sort_badge
        )
    } else {
        format!(" People ({}) {} ", state.people.len(), sort_badge)
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

    lines.push(Line::from(Span::styled(
        " Events",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    if state.detail_events.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no events recorded)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, event) in state.detail_events.iter().enumerate() {
            let is_selected = i == state.detail_event_cursor;
            let date_str = event
                .date
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_else(|| "—".to_string());
            let place_str = state.detail_event_places
                .get(i)
                .and_then(|p| p.as_deref())
                .map(|p| format!("  @ {}", truncate(p, 20)))
                .unwrap_or_default();
            let prefix = if is_selected { "» " } else { "  " };
            let type_style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}{:<13}", prefix, truncate(&event.event_type.to_string(), 13)),
                    type_style,
                ),
                Span::raw(date_str),
                Span::styled(place_str, Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Relationships",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    if state.detail_rel_rows.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no relationships recorded)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, (label, other_name, _rid)) in state.detail_rel_rows.iter().enumerate() {
            let is_selected = i == state.detail_rel_cursor;
            let prefix = if is_selected { "» " } else { "  " };
            let label_style = if is_selected {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Magenta)
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}{:<18}", prefix, truncate(label, 18)),
                    label_style,
                ),
                Span::raw(other_name.as_str()),
            ]));
        }
    }

    // Notes section
    if let Some(ref notes) = state.detail_notes {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Notes",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            " ────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));
        for note_line in notes.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {}", note_line),
                Style::default().fg(Color::White),
            )));
        }
    }

    // Media badge
    if state.detail_media_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Media  ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{} attachment(s)", state.detail_media_count),
                Style::default().fg(Color::Yellow),
            ),
        ]));
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
    if state.task_detail_open {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        draw_tasks_list(frame, state, chunks[0]);
        draw_task_detail(frame, state, chunks[1]);
    } else {
        draw_tasks_list(frame, state, area);
    }

    if state.mode == InputMode::TaskCreate {
        draw_task_create_popup(frame, state, area);
    }
    if state.mode == InputMode::TaskEdit {
        draw_task_edit_popup(frame, state, area);
    }
}

fn draw_tasks_list(frame: &mut Frame, state: &TuiState, area: Rect) {
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

                let desc_style = if task.status == TaskStatus::Done {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                };
                let desc = Span::styled(truncate(&task.description, 56), desc_style);
                ListItem::new(Line::from(vec![status_span, prio_span, desc]))
            }
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.task_rows.is_empty() {
        list_state.select(Some(state.tasks_selected));
    }

    let filter_badge = if state.task_status_filter != TaskStatusFilter::All {
        format!("  [{}]", state.task_status_filter.label())
    } else {
        String::new()
    };
    let visible_count = state.task_rows.iter().filter(|r| matches!(r, TaskRow::Item(_))).count();
    let title = format!(
        " Research Tasks ({}/{}){} ",
        visible_count,
        state.tasks.len(),
        filter_badge
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_task_detail(frame: &mut Frame, state: &TuiState, area: Rect) {
    let task = if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
        state.tasks.get(*idx)
    } else {
        None
    };

    let mut lines: Vec<Line> = vec![Line::from("")];

    if let Some(task) = task {
        // Status + priority badges
        let status_str = match task.status {
            TaskStatus::Pending => "Pending",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Done => "Done",
        };
        let status_color = match task.status {
            TaskStatus::Pending => Color::White,
            TaskStatus::InProgress => Color::Yellow,
            TaskStatus::Done => Color::Green,
        };
        let prio_str = match task.priority {
            TaskPriority::Low => "Low",
            TaskPriority::Medium => "Medium",
            TaskPriority::High => "High",
        };
        let prio_color = match task.priority {
            TaskPriority::Low => Color::DarkGray,
            TaskPriority::Medium => Color::Yellow,
            TaskPriority::High => Color::Red,
        };

        lines.push(Line::from(vec![
            Span::styled("  Status    ", Style::default().fg(Color::Cyan)),
            Span::styled(status_str, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Priority  ", Style::default().fg(Color::Cyan)),
            Span::styled(prio_str, Style::default().fg(prio_color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Description",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            " ────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));
        // Wrap description manually at ~50 chars
        for chunk in task.description.chars().collect::<Vec<_>>().chunks(50) {
            let s: String = chunk.iter().collect();
            lines.push(Line::from(Span::raw(format!("  {}", s))));
        }

        if let Some(ref notes) = task.notes {
            if !notes.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " Notes",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    " ────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
                for note_line in notes.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", note_line),
                        Style::default().fg(Color::White),
                    )));
                }
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Created   ", Style::default().fg(Color::DarkGray)),
            Span::styled(&task.created[..10.min(task.created.len())], Style::default().fg(Color::DarkGray)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "  (no task selected)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Task Detail  Esc / Enter: close ")
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
}

fn draw_task_create_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 56_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 1;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: 3,
    };

    let input_display = format!("{}_", state.task_create_buf);
    let para = Paragraph::new(Line::from(Span::raw(input_display))).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New Task — Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_task_edit_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 60_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 2;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 5 };

    let prio_labels = ["Low ◄", "Medium", "► High"];
    let prio_label = prio_labels[state.task_edit_priority_idx];

    let cursor = "_";
    let desc_text = if state.task_edit_field == 0 {
        format!("{}{}", state.task_edit_desc, cursor)
    } else {
        state.task_edit_desc.clone()
    };
    let prio_text = if state.task_edit_field == 1 {
        format!("◄ {} ►", prio_labels[state.task_edit_priority_idx])
    } else {
        prio_label.to_string()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Desc:     ", Style::default().fg(Color::Cyan)),
            Span::styled(desc_text, if state.task_edit_field == 0 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Priority: ", Style::default().fg(Color::Cyan)),
            Span::styled(prio_text, if state.task_edit_field == 1 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit Task — Tab: switch · ←/→: priority · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

// ── Sources tab ───────────────────────────────────────────────────────────────

fn draw_sources(frame: &mut Frame, state: &TuiState, area: Rect) {
    if state.source_detail_open {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        draw_sources_list(frame, state, chunks[0]);
        draw_source_detail(frame, state, chunks[1]);
    } else {
        draw_sources_list(frame, state, area);
    }

    if state.mode == InputMode::SourceCreate {
        draw_source_create_popup(frame, state, area);
    }
    if state.mode == InputMode::SourceEdit {
        draw_source_edit_popup(frame, state, area);
    }
}

fn draw_source_create_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 56_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 2;
    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: 5,
    };

    let cursor = "_";
    let title_text = if state.source_create_field == 0 {
        format!("{}{}", state.source_create_title, cursor)
    } else {
        state.source_create_title.clone()
    };
    let author_text = if state.source_create_field == 1 {
        format!("{}{}", state.source_create_author, cursor)
    } else {
        state.source_create_author.clone()
    };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Title:   ", Style::default().fg(Color::Cyan)),
            Span::styled(title_text, if state.source_create_field == 0 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Author:  ", Style::default().fg(Color::Cyan)),
            Span::styled(author_text, if state.source_create_field == 1 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" New Source — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Magenta)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_source_edit_popup(frame: &mut Frame, state: &TuiState, area: Rect) {
    let popup_width = 62_u16.min(area.width.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + area.height / 2 - 3;
    let popup_area = Rect { x: popup_x, y: popup_y, width: popup_width, height: 7 };

    let cursor = "_";
    let title_text = if state.source_edit_field == 0 {
        format!("{}{}", state.source_edit_title, cursor)
    } else { state.source_edit_title.clone() };
    let author_text = if state.source_edit_field == 1 {
        format!("{}{}", state.source_edit_author, cursor)
    } else { state.source_edit_author.clone() };
    let year_text = if state.source_edit_field == 2 {
        format!("{}{}", state.source_edit_year, cursor)
    } else { state.source_edit_year.clone() };

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Title:   ", Style::default().fg(Color::Cyan)),
            Span::styled(title_text, if state.source_edit_field == 0 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Author:  ", Style::default().fg(Color::Cyan)),
            Span::styled(author_text, if state.source_edit_field == 1 { active } else { inactive }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Year:    ", Style::default().fg(Color::Cyan)),
            Span::styled(year_text, if state.source_edit_field == 2 { active } else { inactive }),
        ]),
    ];

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Edit Source — Tab: switch · Enter: save · Esc: cancel ")
            .border_style(Style::default().fg(Color::Magenta)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(para, popup_area);
}

fn draw_sources_list(frame: &mut Frame, state: &TuiState, area: Rect) {
    let items: Vec<ListItem> = state
        .filtered_sources
        .iter()
        .map(|&idx| {
            let s = &state.sources[idx];
            let year_str = s
                .year
                .map(|y| format!(" {}", y))
                .unwrap_or_default();
            let cit_str = if s.citation_count > 0 {
                format!(" [{} cit]", s.citation_count)
            } else {
                String::new()
            };
            let line = Line::from(vec![
                Span::raw(truncate(&s.title, 32)),
                Span::styled(year_str, Style::default().fg(Color::Yellow)),
                Span::styled(cit_str, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut list_state = ListState::default();
    if !state.filtered_sources.is_empty() {
        list_state.select(Some(state.sources_selected));
    }

    let title = if !state.source_search_query.is_empty() {
        format!(
            " Sources — \"{}\" ({}/{}) ",
            state.source_search_query,
            state.filtered_sources.len(),
            state.sources.len()
        )
    } else {
        format!(" Sources ({}) ", state.sources.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_source_detail(frame: &mut Frame, state: &TuiState, area: Rect) {
    let source = state.selected_source();
    let title = source
        .map(|s| s.title.as_str())
        .unwrap_or("—");

    let mut lines: Vec<Line> = Vec::new();

    if let Some(s) = source {
        if let Some(ref author) = s.author {
            lines.push(Line::from(vec![
                Span::styled("  Author  ", Style::default().fg(Color::Cyan)),
                Span::raw(author.as_str()),
            ]));
        }
        if let Some(year) = s.year {
            lines.push(Line::from(vec![
                Span::styled("  Year    ", Style::default().fg(Color::Cyan)),
                Span::styled(year.to_string(), Style::default().fg(Color::Yellow)),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        " Citations",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    if state.source_detail_citations.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no citations)",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (event_label, page) in &state.source_detail_citations {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<24}", truncate(event_label, 24)),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(page.as_str(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }

    let scroll = state.source_detail_scroll as u16;

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", truncate(title, 38))),
        )
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, area);
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
            // Derived metrics
            if s.people > 0 {
                let avg = s.events as f64 / s.people as f64;
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<18}", "Avg events/person"),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{:.1}", avg),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            if s.events > 0 {
                let pct = s.citations * 100 / s.events;
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("    {:<18}", "Citation coverage"),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{}%", pct),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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

    // ── Research tasks breakdown ─────────────────────────────────────────────
    let total_tasks = state.tasks_pending + state.tasks_in_progress + state.tasks_done;
    if total_tasks > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "    Research Tasks",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));

        let task_rows: [(&str, usize, Color); 3] = [
            ("In Progress", state.tasks_in_progress, Color::Yellow),
            ("Pending", state.tasks_pending, Color::White),
            ("Done", state.tasks_done, Color::Green),
        ];
        for (label, count, color) in &task_rows {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("      {:<14}", label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    count.to_string(),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Progress bar: done / total
        let pct = (state.tasks_done * 100) / total_tasks.max(1);
        let bar_width = 20usize;
        let filled = (bar_width * state.tasks_done) / total_tasks.max(1);
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
        lines.push(Line::from(vec![
            Span::styled("      Progress      ", Style::default().fg(Color::DarkGray)),
            Span::styled(bar, Style::default().fg(Color::Green)),
            Span::styled(
                format!("  {}%", pct),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // ── Top places ───────────────────────────────────────────────────────────
    if !state.top_places.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "    Top Places",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for (name, count) in &state.top_places {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("      {:<22}", truncate(name, 22)),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} event{}", count, if *count == 1 { "" } else { "s" }),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
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

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Database Statistics "),
    );

    frame.render_widget(para, area);
}

// ── Status bar ────────────────────────────────────────────────────────────────

fn draw_status(frame: &mut Frame, state: &TuiState, area: Rect) {
    let text = match state.mode {
        InputMode::Search => if state.active_tab == Tab::Sources {
            format!(" ESC: cancel  Backspace: delete  [{} match(es)]", state.filtered_sources.len())
        } else {
            format!(" ESC: cancel  Backspace: delete  [{} match(es)]", state.filtered_people.len())
        },
        InputMode::TaskCreate => " Type task description  Enter: save  Esc: cancel".to_string(),
        InputMode::TaskEdit => " Tab: switch field  ←/→: priority  Enter: save  Esc: cancel".to_string(),
        InputMode::PersonCreate => " Tab: switch field  Enter: save  Esc: cancel".to_string(),
        InputMode::PersonEdit => " Tab: switch field  Enter: save  Esc: cancel".to_string(),
        InputMode::SourceCreate => " Tab: switch field  Enter: save  Esc: cancel".to_string(),
        InputMode::ConfirmDelete => " y: confirm delete  any other key: cancel".to_string(),
        InputMode::EventCreate => " Tab: next field  ←/→: cycle type  Enter: save  Esc: cancel".to_string(),
        InputMode::RelationshipCreate => " Tab: switch field  ←/→: cycle type  Enter: save  Esc: cancel".to_string(),
        InputMode::SourceEdit => " Tab: switch field  Enter: save  Esc: cancel".to_string(),
        InputMode::PersonNotesEdit => " Type notes  Enter: save  Esc: cancel".to_string(),
        InputMode::EventEdit => " Tab: next field  ←/→: cycle type  Enter: save  Esc: cancel".to_string(),
        InputMode::Normal => match state.active_tab {
            Tab::People => {
                if state.detail_open {
                    " ↑↓/jk: select event  E: edit event  x: delete event  a: add  r: add rel  e: name  N: notes  Esc: close".to_string()
                } else {
                    " Tab: next  ↑↓/jk  g/G  n: new  e: edit  N: notes  s: sort  x: delete  /: search  Enter: detail  q: quit"
                        .to_string()
                }
            }
            Tab::Tasks => {
                if state.task_detail_open {
                    " Enter/Esc: close detail  ↑↓/jk  d/c: done  e: edit  p: priority  Tab: next  q: quit".to_string()
                } else {
                    " Tab: next  ↑↓/jk  g/G  n: new  e: edit  d/c: done  p: priority  x: delete  f: filter  Enter: detail  q: quit".to_string()
                }
            }
            Tab::Sources => {
                if state.source_detail_open {
                    " ESC/Enter: close  ↑↓/jk: scroll  Tab: next  q: quit".to_string()
                } else {
                    " Tab: next  ↑↓/jk  g/G  n: new  e: edit  x: delete  /: search  Enter: citations  q: quit".to_string()
                }
            }
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
