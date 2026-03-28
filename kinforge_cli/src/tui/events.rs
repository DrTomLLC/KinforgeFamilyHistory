use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use kinforge_core::models::{EventId, PersonId, RelationshipId, Sex, SourceId, TaskId};

use super::state::{
    build_filtered_task_rows, first_task_item_idx, InputMode, Tab, TaskRow,
    TuiState, TUI_EVENT_TYPES, TUI_REL_TYPES,
};

pub enum Action {
    None,
    Quit,
    OpenPersonDetail(PersonId),
    OpenSourceDetail(SourceId),
    CompleteTask(TaskId),
    CreateTask(String),
    CycleTaskPriority(TaskId),
    DeleteTask(TaskId),
    CreatePerson(String, String, Sex),  // (given, surname, sex)
    EditPerson(PersonId, String, String), // (id, given, surname)
    CreateSource(String, String),  // (title, author)
    DeletePerson(PersonId),
    CreateEvent(PersonId, String, String, String), // (person_id, type_name, date_str, place_str)
    DeleteSource(SourceId),
    EditTask(TaskId, String, kinforge_core::models::TaskPriority),
    CreateRelationship(PersonId, String, PersonId), // (person1, rel_type_token, person2)
    EditSource(SourceId, String, String, Option<i32>), // (id, title, author, year)
    UpdatePersonNotes(PersonId, String),                // (id, notes text)
    EditEvent(EventId, String, String, String),         // (id, type_name, date_str, place_str)
    DeleteEvent(EventId),
    DeleteRelationship(RelationshipId),
    StartTask(TaskId),
    AddCitation(EventId, SourceId, String), // (event_id, source_id, page)
    UpdateTaskNotes(TaskId, String),        // (id, notes text; empty = clear)
}

pub fn handle_key(state: &mut TuiState, key: KeyEvent) -> Action {
    // Only react to key-press events
    if key.kind != KeyEventKind::Press {
        return Action::None;
    }

    // Ctrl+C always quits regardless of mode
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    match state.mode {
        InputMode::Normal => handle_normal(state, key),
        InputMode::Search => handle_search(state, key),
        InputMode::TaskCreate => handle_task_create(state, key),
        InputMode::PersonCreate => handle_person_create(state, key),
        InputMode::PersonEdit => handle_person_edit(state, key),
        InputMode::SourceCreate => handle_source_create(state, key),
        InputMode::ConfirmDelete => handle_confirm_delete(state, key),
        InputMode::EventCreate => handle_event_create(state, key),
        InputMode::TaskEdit => handle_task_edit(state, key),
        InputMode::RelationshipCreate => handle_rel_create(state, key),
        InputMode::SourceEdit => handle_source_edit(state, key),
        InputMode::PersonNotesEdit => handle_person_notes_edit(state, key),
        InputMode::EventEdit => handle_event_edit(state, key),
        InputMode::CitationCreate => handle_citation_create(state, key),
        InputMode::Help => handle_help(state, key),
        InputMode::TaskNotesEdit => handle_task_notes_edit(state, key),
    }
}

fn handle_normal(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,

        KeyCode::Tab => {
            state.active_tab = state.active_tab.next();
            // Close open detail panels when switching tabs
            state.detail_open = false;
            state.source_detail_open = false;
            state.task_detail_open = false;
            Action::None
        }
        KeyCode::BackTab => {
            state.active_tab = state.active_tab.prev();
            state.detail_open = false;
            state.source_detail_open = false;
            state.task_detail_open = false;
            Action::None
        }

        KeyCode::Up | KeyCode::Char('k') => {
            match state.active_tab {
                Tab::People => {
                    if state.detail_open {
                        if !state.detail_events.is_empty() {
                            state.detail_event_cursor =
                                state.detail_event_cursor.saturating_sub(1);
                        } else {
                            state.detail_scroll = state.detail_scroll.saturating_sub(1);
                        }
                    } else if state.people_selected > 0 {
                        state.people_selected -= 1;
                    }
                }
                Tab::Tasks => move_task_up(state),
                Tab::Sources => {
                    if state.source_detail_open {
                        state.source_detail_scroll =
                            state.source_detail_scroll.saturating_sub(1);
                    } else if state.sources_selected > 0 {
                        state.sources_selected -= 1;
                    }
                }
                Tab::Stats => {}
            }
            Action::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            match state.active_tab {
                Tab::People => {
                    if state.detail_open {
                        if !state.detail_events.is_empty() {
                            let max = state.detail_events.len() - 1;
                            if state.detail_event_cursor < max {
                                state.detail_event_cursor += 1;
                            }
                        } else {
                            state.detail_scroll = state.detail_scroll.saturating_add(1);
                        }
                    } else {
                        let max = state.filtered_people.len().saturating_sub(1);
                        if state.people_selected < max {
                            state.people_selected += 1;
                        }
                    }
                }
                Tab::Tasks => move_task_down(state),
                Tab::Sources => {
                    if state.source_detail_open {
                        state.source_detail_scroll =
                            state.source_detail_scroll.saturating_add(1);
                    } else {
                        let max = state.sources.len().saturating_sub(1);
                        if state.sources_selected < max {
                            state.sources_selected += 1;
                        }
                    }
                }
                Tab::Stats => {}
            }
            Action::None
        }

        KeyCode::Char('/') if state.active_tab == Tab::People && !state.detail_open => {
            state.search_active = true;
            state.mode = InputMode::Search;
            Action::None
        }

        KeyCode::Char('/') if state.active_tab == Tab::Sources && !state.source_detail_open => {
            state.mode = InputMode::Search;
            Action::None
        }

        // Jump to top
        KeyCode::Char('g') => {
            match state.active_tab {
                Tab::People if !state.detail_open => { state.people_selected = 0; }
                Tab::Tasks => { state.tasks_selected = first_task_item_idx(&state.task_rows); }
                Tab::Sources if !state.source_detail_open => { state.sources_selected = 0; }
                _ => {}
            }
            Action::None
        }

        // Jump to bottom
        KeyCode::Char('G') => {
            match state.active_tab {
                Tab::People if !state.detail_open => {
                    state.people_selected = state.filtered_people.len().saturating_sub(1);
                }
                Tab::Tasks => {
                    if let Some(idx) = state.task_rows.iter().rposition(|r| matches!(r, TaskRow::Item(_))) {
                        state.tasks_selected = idx;
                    }
                }
                Tab::Sources if !state.source_detail_open => {
                    state.sources_selected = state.sources.len().saturating_sub(1);
                }
                _ => {}
            }
            Action::None
        }

        // Page up (10 items)
        KeyCode::PageUp => {
            match state.active_tab {
                Tab::People if !state.detail_open => {
                    state.people_selected = state.people_selected.saturating_sub(10);
                }
                Tab::Tasks => { for _ in 0..10 { move_task_up(state); } }
                Tab::Sources if !state.source_detail_open => {
                    state.sources_selected = state.sources_selected.saturating_sub(10);
                }
                _ => {}
            }
            Action::None
        }

        // Page down (10 items)
        KeyCode::PageDown => {
            match state.active_tab {
                Tab::People if !state.detail_open => {
                    let max = state.filtered_people.len().saturating_sub(1);
                    state.people_selected = (state.people_selected + 10).min(max);
                }
                Tab::Tasks => { for _ in 0..10 { move_task_down(state); } }
                Tab::Sources if !state.source_detail_open => {
                    let max = state.sources.len().saturating_sub(1);
                    state.sources_selected = (state.sources_selected + 10).min(max);
                }
                _ => {}
            }
            Action::None
        }

        KeyCode::Enter => match state.active_tab {
            Tab::People => {
                if state.detail_open {
                    // Close detail on second Enter
                    state.detail_open = false;
                    state.detail_scroll = 0;
                    Action::None
                } else if let Some(row) = state.selected_person() {
                    Action::OpenPersonDetail(row.id.clone())
                } else {
                    Action::None
                }
            }
            Tab::Tasks => {
                if state.task_detail_open {
                    state.task_detail_open = false;
                } else if matches!(state.task_rows.get(state.tasks_selected), Some(TaskRow::Item(_))) {
                    state.task_detail_open = true;
                }
                Action::None
            }
            Tab::Sources => {
                if state.source_detail_open {
                    state.source_detail_open = false;
                    state.source_detail_scroll = 0;
                    Action::None
                } else if let Some(row) = state.selected_source() {
                    Action::OpenSourceDetail(row.id.clone())
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        },

        // New source: press 'n' in Sources tab (when no detail open)
        KeyCode::Char('n') if state.active_tab == Tab::Sources && !state.source_detail_open => {
            state.source_create_title.clear();
            state.source_create_author.clear();
            state.source_create_field = 0;
            state.mode = InputMode::SourceCreate;
            Action::None
        }

        // Edit source: press 'e' in Sources tab (when no detail open)
        KeyCode::Char('e') if state.active_tab == Tab::Sources && !state.source_detail_open => {
            let info = state.selected_source().map(|s| {
                (s.id.clone(), s.title.clone(), s.author.clone().unwrap_or_default(), s.year)
            });
            if let Some((sid, title, author, year)) = info {
                state.source_edit_id = Some(sid);
                state.source_edit_title = title;
                state.source_edit_author = author;
                state.source_edit_year = year.map(|y| y.to_string()).unwrap_or_default();
                state.source_edit_field = 0;
                state.mode = InputMode::SourceEdit;
            }
            Action::None
        }

        // Delete source: press 'x' in Sources tab (when no detail open)
        KeyCode::Char('x') if state.active_tab == Tab::Sources && !state.source_detail_open => {
            let info = state.selected_source().map(|s| (s.title.clone(), s.id.clone()));
            if let Some((title, sid)) = info {
                state.confirm_name = title;
                state.confirm_source_id = Some(sid);
                state.confirm_person_id = None;
                state.mode = InputMode::ConfirmDelete;
            }
            Action::None
        }

        // New person: press 'n' in People tab (when no detail/search open)
        KeyCode::Char('n') if state.active_tab == Tab::People && !state.detail_open && !state.search_active => {
            state.person_create_given.clear();
            state.person_create_surname.clear();
            state.person_create_field = 0;
            state.mode = InputMode::PersonCreate;
            Action::None
        }

        // Edit person primary name: press 'e' in People tab
        KeyCode::Char('e') if state.active_tab == Tab::People => {
            let info = if state.detail_open {
                state.detail_person_id.as_ref().and_then(|pid| {
                    state.people.iter().find(|p| &p.id == pid)
                        .map(|p| (p.id.clone(), p.display_name.clone()))
                })
            } else {
                state.selected_person().map(|r| (r.id.clone(), r.display_name.clone()))
            };
            if let Some((pid, _)) = info {
                // Prefill from the PersonRow display_name — parse "Given Surname" split
                let row = state.people.iter().find(|p| p.id == pid).cloned();
                let (given, surname) = if let Some(r) = &row {
                    // Try to get given/surname from app via the PersonRow — we use display_name as fallback
                    // Split on last space: everything before = given, last word = surname
                    let dn = r.display_name.trim();
                    if let Some(pos) = dn.rfind(' ') {
                        (dn[..pos].to_string(), dn[pos + 1..].to_string())
                    } else {
                        (String::new(), dn.to_string())
                    }
                } else {
                    (String::new(), String::new())
                };
                state.person_edit_given = given;
                state.person_edit_surname = surname;
                state.person_edit_field = 0;
                state.person_edit_id = Some(pid);
                state.mode = InputMode::PersonEdit;
            }
            Action::None
        }

        // Add event: press 'a' in People tab when detail is open
        KeyCode::Char('a') if state.active_tab == Tab::People && state.detail_open => {
            if let Some(pid) = state.detail_person_id.clone() {
                state.event_create_type_idx = 0;
                state.event_create_date.clear();
                state.event_create_place.clear();
                state.event_create_field = 0;
                state.event_create_person_id = Some(pid);
                state.mode = InputMode::EventCreate;
            }
            Action::None
        }

        // Edit selected event: press 'E' (shift+e) in People tab when detail is open
        KeyCode::Char('E') if state.active_tab == Tab::People && state.detail_open => {
            let idx = state.detail_event_cursor;
            if let Some(evt) = state.detail_events.get(idx) {
                let type_name = evt.event_type.to_string();
                let type_idx = TUI_EVENT_TYPES.iter()
                    .position(|&t| t.eq_ignore_ascii_case(&type_name))
                    .unwrap_or(0);
                let date_str = evt.date.as_ref().map(|d| {
                    match d {
                        kinforge_core::models::EventDate::Exact(nd)
                        | kinforge_core::models::EventDate::Approximate(nd) =>
                            nd.format("%Y-%m-%d").to_string(),
                        _ => String::new(),
                    }
                }).unwrap_or_default();
                let place_str = state.detail_event_places
                    .get(idx).and_then(|p| p.as_deref()).unwrap_or("").to_string();
                state.event_edit_id = Some(evt.id.clone());
                state.event_edit_type_idx = type_idx;
                state.event_edit_date = date_str;
                state.event_edit_place = place_str;
                state.event_edit_field = 0;
                state.mode = InputMode::EventEdit;
            }
            Action::None
        }

        // Delete selected event: press 'x' in People tab when detail is open
        KeyCode::Char('x') if state.active_tab == Tab::People && state.detail_open => {
            let idx = state.detail_event_cursor;
            if let Some(evt) = state.detail_events.get(idx) {
                let label = evt.event_type.to_string();
                state.confirm_name = label;
                state.confirm_event_id = Some(evt.id.clone());
                state.confirm_person_id = None;
                state.confirm_source_id = None;
                state.mode = InputMode::ConfirmDelete;
            }
            Action::None
        }

        // Move relationship cursor up: press 'J' in People detail
        KeyCode::Char('J') if state.active_tab == Tab::People && state.detail_open => {
            if !state.detail_rel_rows.is_empty() {
                let max = state.detail_rel_rows.len() - 1;
                if state.detail_rel_cursor < max {
                    state.detail_rel_cursor += 1;
                }
            }
            Action::None
        }

        // Move relationship cursor up: press 'K' in People detail
        KeyCode::Char('K') if state.active_tab == Tab::People && state.detail_open => {
            state.detail_rel_cursor = state.detail_rel_cursor.saturating_sub(1);
            Action::None
        }

        // Delete selected relationship: press 'X' in People detail
        KeyCode::Char('X') if state.active_tab == Tab::People && state.detail_open => {
            let idx = state.detail_rel_cursor;
            if let Some((label, other_name, rid)) = state.detail_rel_rows.get(idx) {
                let display = format!("{} {}", label, other_name);
                state.confirm_name = display;
                state.confirm_rel_id = Some(rid.clone());
                state.confirm_person_id = None;
                state.confirm_source_id = None;
                state.confirm_event_id = None;
                state.mode = InputMode::ConfirmDelete;
            }
            Action::None
        }

        // Cite selected event: press 'C' in People detail
        KeyCode::Char('C') if state.active_tab == Tab::People && state.detail_open => {
            let idx = state.detail_event_cursor;
            if let Some(evt) = state.detail_events.get(idx) {
                state.citation_event_id = Some(evt.id.clone());
                state.citation_source_buf.clear();
                state.citation_page_buf.clear();
                state.citation_field = 0;
                // Populate all sources as initial matches
                state.citation_source_matches = state.sources.iter()
                    .map(|s| (s.id.clone(), s.title.clone()))
                    .collect();
                state.citation_source_cursor = 0;
                state.mode = InputMode::CitationCreate;
            }
            Action::None
        }

        // Edit person notes: press 'N' in People tab (list or detail)
        KeyCode::Char('N') if state.active_tab == Tab::People => {
            let pid = if state.detail_open {
                state.detail_person_id.clone()
            } else {
                state.selected_person().map(|r| r.id.clone())
            };
            if let Some(pid) = pid {
                let existing = state.detail_notes.clone().unwrap_or_default();
                state.person_notes_buf = existing;
                state.person_notes_id = Some(pid);
                state.mode = InputMode::PersonNotesEdit;
            }
            Action::None
        }

        // Add relationship: press 'r' in People tab when detail is open
        KeyCode::Char('r') if state.active_tab == Tab::People && state.detail_open => {
            if let Some(pid) = state.detail_person_id.clone() {
                state.rel_create_person2_buf.clear();
                state.rel_create_type_idx = 0;
                state.rel_create_field = 0;
                state.rel_create_person1_id = Some(pid);
                state.mode = InputMode::RelationshipCreate;
            }
            Action::None
        }

        // Delete person: press 'x' in People tab (opens confirm popup)
        KeyCode::Char('x') if state.active_tab == Tab::People && !state.detail_open => {
            let info = state.selected_person().map(|r| (r.display_name.clone(), r.id.clone()));
            if let Some((name, pid)) = info {
                state.confirm_name = name;
                state.confirm_person_id = Some(pid);
                state.mode = InputMode::ConfirmDelete;
            }
            Action::None
        }

        // Toggle sort order: press 's' in People tab
        KeyCode::Char('s') if state.active_tab == Tab::People && !state.detail_open && !state.search_active => {
            state.sort_order = state.sort_order.next();
            state.recompute_filter();
            Action::None
        }

        // Help popup: press '?' anywhere in Normal mode
        KeyCode::Char('?') => {
            state.mode = InputMode::Help;
            Action::None
        }

        // Edit task notes: press 'N' in Tasks tab
        KeyCode::Char('N') if state.active_tab == Tab::Tasks => {
            let info = if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                Some((task.id.clone(), task.notes.clone().unwrap_or_default()))
            } else {
                None
            };
            if let Some((tid, existing_notes)) = info {
                state.task_notes_buf = existing_notes;
                state.task_notes_id = Some(tid);
                state.mode = InputMode::TaskNotesEdit;
            }
            Action::None
        }

        // Task quick-complete: press 'd' or 'c' on a task
        KeyCode::Char('d') | KeyCode::Char('c') if state.active_tab == Tab::Tasks => {
            if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                if task.status != kinforge_core::models::TaskStatus::Done {
                    return Action::CompleteTask(task.id.clone());
                }
            }
            Action::None
        }

        // New task: press 'n' in Tasks tab
        KeyCode::Char('n') if state.active_tab == Tab::Tasks => {
            state.task_create_buf.clear();
            state.mode = InputMode::TaskCreate;
            Action::None
        }

        // Edit task: press 'e' in Tasks tab
        KeyCode::Char('e') if state.active_tab == Tab::Tasks => {
            if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                state.task_edit_desc = task.description.clone();
                state.task_edit_priority_idx = match task.priority {
                    kinforge_core::models::TaskPriority::Low => 0,
                    kinforge_core::models::TaskPriority::Medium => 1,
                    kinforge_core::models::TaskPriority::High => 2,
                };
                state.task_edit_id = Some(task.id.clone());
                state.task_edit_field = 0;
                state.mode = InputMode::TaskEdit;
            }
            Action::None
        }

        // Filter tasks by status: press 'f' in Tasks tab
        KeyCode::Char('f') if state.active_tab == Tab::Tasks => {
            state.task_status_filter = state.task_status_filter.next();
            state.task_rows = build_filtered_task_rows(&state.tasks, state.task_status_filter);
            state.tasks_selected = first_task_item_idx(&state.task_rows);
            Action::None
        }

        // Cycle task priority: press 'p' on a task
        KeyCode::Char('p') if state.active_tab == Tab::Tasks => {
            if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                return Action::CycleTaskPriority(task.id.clone());
            }
            Action::None
        }

        // Delete task: press 'x' on a task
        KeyCode::Char('x') if state.active_tab == Tab::Tasks => {
            if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                return Action::DeleteTask(task.id.clone());
            }
            Action::None
        }

        // Start task (set InProgress): press 's' on a task
        KeyCode::Char('s') if state.active_tab == Tab::Tasks => {
            if let Some(TaskRow::Item(idx)) = state.task_rows.get(state.tasks_selected) {
                let task = &state.tasks[*idx];
                if task.status == kinforge_core::models::TaskStatus::Pending {
                    return Action::StartTask(task.id.clone());
                }
            }
            Action::None
        }

        KeyCode::Esc => {
            if state.detail_open {
                state.detail_open = false;
                state.detail_scroll = 0;
            } else if state.source_detail_open {
                state.source_detail_open = false;
                state.source_detail_scroll = 0;
            } else if state.task_detail_open {
                state.task_detail_open = false;
            }
            Action::None
        }

        _ => Action::None,
    }
}

fn handle_search(state: &mut TuiState, key: KeyEvent) -> Action {
    let in_sources = state.active_tab == Tab::Sources;
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            if in_sources {
                state.source_search_query.clear();
                state.recompute_source_filter();
            } else {
                state.search_active = false;
                state.search_query.clear();
                state.recompute_filter();
            }
        }
        KeyCode::Enter => {
            state.mode = InputMode::Normal;
            if in_sources {
                // keep filter active, just exit typing
            } else {
                state.search_active = false;
                if let Some(row) = state.selected_person() {
                    return Action::OpenPersonDetail(row.id.clone());
                }
            }
        }
        KeyCode::Backspace => {
            if in_sources {
                state.source_search_query.pop();
                state.recompute_source_filter();
            } else {
                state.search_query.pop();
                state.recompute_filter();
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if in_sources {
                state.sources_selected = state.sources_selected.saturating_sub(1);
            } else if state.people_selected > 0 {
                state.people_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if in_sources {
                let max = state.filtered_sources.len().saturating_sub(1);
                if state.sources_selected < max { state.sources_selected += 1; }
            } else {
                let max = state.filtered_people.len().saturating_sub(1);
                if state.people_selected < max { state.people_selected += 1; }
            }
        }
        KeyCode::Char(c) => {
            if in_sources {
                state.source_search_query.push(c);
                state.recompute_source_filter();
            } else {
                state.search_query.push(c);
                state.recompute_filter();
            }
        }
        _ => {}
    }
    Action::None
}

fn handle_task_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            let desc = state.task_create_buf.trim().to_string();
            state.mode = InputMode::Normal;
            if !desc.is_empty() {
                return Action::CreateTask(desc);
            }
        }
        KeyCode::Backspace => {
            state.task_create_buf.pop();
        }
        KeyCode::Char(c) => {
            state.task_create_buf.push(c);
        }
        _ => {}
    }
    Action::None
}

const SEX_OPTIONS: [Sex; 3] = [Sex::Unknown, Sex::Male, Sex::Female];

fn handle_person_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
        }
        KeyCode::Tab => {
            state.person_create_field = (state.person_create_field + 1) % 3;
        }
        KeyCode::BackTab => {
            state.person_create_field = (state.person_create_field + 2) % 3;
        }
        KeyCode::Left if state.person_create_field == 2 => {
            state.person_create_sex = (state.person_create_sex + 2) % 3;
        }
        KeyCode::Right if state.person_create_field == 2 => {
            state.person_create_sex = (state.person_create_sex + 1) % 3;
        }
        KeyCode::Enter => {
            let given = state.person_create_given.trim().to_string();
            let surname = state.person_create_surname.trim().to_string();
            let sex = SEX_OPTIONS[state.person_create_sex as usize].clone();
            state.mode = InputMode::Normal;
            if !given.is_empty() || !surname.is_empty() {
                return Action::CreatePerson(given, surname, sex);
            }
        }
        KeyCode::Backspace => {
            match state.person_create_field {
                0 => { state.person_create_given.pop(); }
                1 => { state.person_create_surname.pop(); }
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            match state.person_create_field {
                0 => state.person_create_given.push(c),
                1 => state.person_create_surname.push(c),
                _ => {}
            }
        }
        _ => {}
    }
    Action::None
}

fn handle_person_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.person_edit_id = None;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.person_edit_field = 1 - state.person_edit_field;
        }
        KeyCode::Enter => {
            let given = state.person_edit_given.trim().to_string();
            let surname = state.person_edit_surname.trim().to_string();
            if let Some(pid) = state.person_edit_id.take() {
                state.mode = InputMode::Normal;
                if !given.is_empty() || !surname.is_empty() {
                    return Action::EditPerson(pid, given, surname);
                }
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            if state.person_edit_field == 0 {
                state.person_edit_given.pop();
            } else {
                state.person_edit_surname.pop();
            }
        }
        KeyCode::Char(c) => {
            if state.person_edit_field == 0 {
                state.person_edit_given.push(c);
            } else {
                state.person_edit_surname.push(c);
            }
        }
        _ => {}
    }
    Action::None
}

fn handle_confirm_delete(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            state.confirm_name.clear();
            state.mode = InputMode::Normal;
            if let Some(pid) = state.confirm_person_id.take() {
                return Action::DeletePerson(pid);
            }
            if let Some(sid) = state.confirm_source_id.take() {
                return Action::DeleteSource(sid);
            }
            if let Some(eid) = state.confirm_event_id.take() {
                return Action::DeleteEvent(eid);
            }
            if let Some(rid) = state.confirm_rel_id.take() {
                return Action::DeleteRelationship(rid);
            }
        }
        _ => {
            state.confirm_person_id = None;
            state.confirm_source_id = None;
            state.confirm_event_id = None;
            state.confirm_rel_id = None;
            state.confirm_name.clear();
            state.mode = InputMode::Normal;
        }
    }
    Action::None
}

fn handle_source_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.source_create_field = 1 - state.source_create_field;
        }
        KeyCode::Enter => {
            let title = state.source_create_title.trim().to_string();
            let author = state.source_create_author.trim().to_string();
            state.mode = InputMode::Normal;
            if !title.is_empty() {
                return Action::CreateSource(title, author);
            }
        }
        KeyCode::Backspace => {
            if state.source_create_field == 0 {
                state.source_create_title.pop();
            } else {
                state.source_create_author.pop();
            }
        }
        KeyCode::Char(c) => {
            if state.source_create_field == 0 {
                state.source_create_title.push(c);
            } else {
                state.source_create_author.push(c);
            }
        }
        _ => {}
    }
    Action::None
}

fn handle_task_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    use kinforge_core::models::TaskPriority;
    const PRIOS: [TaskPriority; 3] = [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High];
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.task_edit_id = None;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.task_edit_field = 1 - state.task_edit_field;
        }
        // Left/Right cycle priority when on field 1
        KeyCode::Left if state.task_edit_field == 1 => {
            state.task_edit_priority_idx =
                state.task_edit_priority_idx.saturating_sub(1);
        }
        KeyCode::Right if state.task_edit_field == 1 => {
            if state.task_edit_priority_idx < 2 {
                state.task_edit_priority_idx += 1;
            }
        }
        KeyCode::Enter => {
            let desc = state.task_edit_desc.trim().to_string();
            let priority = PRIOS[state.task_edit_priority_idx].clone();
            if let Some(tid) = state.task_edit_id.take() {
                state.mode = InputMode::Normal;
                if !desc.is_empty() {
                    return Action::EditTask(tid, desc, priority);
                }
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace if state.task_edit_field == 0 => {
            state.task_edit_desc.pop();
        }
        KeyCode::Char(c) if state.task_edit_field == 0 => {
            state.task_edit_desc.push(c);
        }
        _ => {}
    }
    Action::None
}

fn handle_event_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.event_create_person_id = None;
        }
        KeyCode::Tab => {
            state.event_create_field = (state.event_create_field + 1) % 3;
        }
        KeyCode::BackTab => {
            state.event_create_field = (state.event_create_field + 2) % 3;
        }
        // Left/Right cycle event type when on field 0
        KeyCode::Left if state.event_create_field == 0 => {
            if state.event_create_type_idx == 0 {
                state.event_create_type_idx = TUI_EVENT_TYPES.len() - 1;
            } else {
                state.event_create_type_idx -= 1;
            }
        }
        KeyCode::Right if state.event_create_field == 0 => {
            state.event_create_type_idx =
                (state.event_create_type_idx + 1) % TUI_EVENT_TYPES.len();
        }
        KeyCode::Enter => {
            let type_name = TUI_EVENT_TYPES[state.event_create_type_idx].to_string();
            let date_str = state.event_create_date.trim().to_string();
            let place_str = state.event_create_place.trim().to_string();
            if let Some(pid) = state.event_create_person_id.take() {
                state.mode = InputMode::Normal;
                return Action::CreateEvent(pid, type_name, date_str, place_str);
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => match state.event_create_field {
            1 => { state.event_create_date.pop(); }
            2 => { state.event_create_place.pop(); }
            _ => {}
        },
        KeyCode::Char(c) => match state.event_create_field {
            1 => { state.event_create_date.push(c); }
            2 => { state.event_create_place.push(c); }
            _ => {}
        },
        _ => {}
    }
    Action::None
}

fn move_task_up(state: &mut TuiState) {
    let cur = state.tasks_selected;
    let mut i = cur;
    loop {
        if i == 0 {
            break;
        }
        i -= 1;
        if matches!(state.task_rows[i], TaskRow::Item(_)) {
            state.tasks_selected = i;
            break;
        }
    }
}

fn move_task_down(state: &mut TuiState) {
    let cur = state.tasks_selected;
    let mut i = cur + 1;
    while i < state.task_rows.len() {
        if matches!(state.task_rows[i], TaskRow::Item(_)) {
            state.tasks_selected = i;
            break;
        }
        i += 1;
    }
}

fn handle_source_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.source_edit_id = None;
        }
        KeyCode::Tab => {
            state.source_edit_field = (state.source_edit_field + 1) % 3;
        }
        KeyCode::BackTab => {
            state.source_edit_field = (state.source_edit_field + 2) % 3;
        }
        KeyCode::Enter => {
            if let Some(sid) = state.source_edit_id.take() {
                let title = state.source_edit_title.trim().to_string();
                if !title.is_empty() {
                    let author = state.source_edit_author.trim().to_string();
                    let year = state.source_edit_year.trim().parse::<i32>().ok();
                    state.mode = InputMode::Normal;
                    return Action::EditSource(sid, title, author, year);
                }
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            match state.source_edit_field {
                0 => { state.source_edit_title.pop(); }
                1 => { state.source_edit_author.pop(); }
                2 => { state.source_edit_year.pop(); }
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            match state.source_edit_field {
                0 => state.source_edit_title.push(c),
                1 => state.source_edit_author.push(c),
                2 if c.is_ascii_digit() || c == '-' => state.source_edit_year.push(c),
                _ => {}
            }
        }
        _ => {}
    }
    Action::None
}

fn handle_event_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.event_edit_id = None;
        }
        KeyCode::Tab => {
            state.event_edit_field = (state.event_edit_field + 1) % 3;
        }
        KeyCode::BackTab => {
            state.event_edit_field = (state.event_edit_field + 2) % 3;
        }
        KeyCode::Left if state.event_edit_field == 0 => {
            if state.event_edit_type_idx == 0 {
                state.event_edit_type_idx = TUI_EVENT_TYPES.len() - 1;
            } else {
                state.event_edit_type_idx -= 1;
            }
        }
        KeyCode::Right if state.event_edit_field == 0 => {
            state.event_edit_type_idx =
                (state.event_edit_type_idx + 1) % TUI_EVENT_TYPES.len();
        }
        KeyCode::Enter => {
            if let Some(eid) = state.event_edit_id.take() {
                let type_name = TUI_EVENT_TYPES[state.event_edit_type_idx].to_string();
                let date_str = state.event_edit_date.trim().to_string();
                let place_str = state.event_edit_place.trim().to_string();
                state.mode = InputMode::Normal;
                return Action::EditEvent(eid, type_name, date_str, place_str);
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => match state.event_edit_field {
            1 => { state.event_edit_date.pop(); }
            2 => { state.event_edit_place.pop(); }
            _ => {}
        },
        KeyCode::Char(c) => match state.event_edit_field {
            1 => { state.event_edit_date.push(c); }
            2 => { state.event_edit_place.push(c); }
            _ => {}
        },
        _ => {}
    }
    Action::None
}

fn handle_person_notes_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.person_notes_id = None;
        }
        KeyCode::Enter => {
            if let Some(pid) = state.person_notes_id.take() {
                let notes = state.person_notes_buf.trim().to_string();
                state.mode = InputMode::Normal;
                return Action::UpdatePersonNotes(pid, notes);
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            state.person_notes_buf.pop();
        }
        KeyCode::Char(c) => {
            state.person_notes_buf.push(c);
        }
        _ => {}
    }
    Action::None
}

fn handle_rel_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.rel_create_person1_id = None;
        }
        KeyCode::Tab => {
            state.rel_create_field = 1 - state.rel_create_field;
        }
        KeyCode::BackTab => {
            state.rel_create_field = 1 - state.rel_create_field;
        }
        KeyCode::Left if state.rel_create_field == 1 => {
            if state.rel_create_type_idx == 0 {
                state.rel_create_type_idx = TUI_REL_TYPES.len() - 1;
            } else {
                state.rel_create_type_idx -= 1;
            }
        }
        KeyCode::Right if state.rel_create_field == 1 => {
            state.rel_create_type_idx = (state.rel_create_type_idx + 1) % TUI_REL_TYPES.len();
        }
        KeyCode::Enter => {
            if let Some(pid1) = state.rel_create_person1_id.take() {
                let query = state.rel_create_person2_buf.to_lowercase();
                let pid2 = state.people.iter()
                    .find(|p| p.display_name.to_lowercase().contains(&query) && p.id != pid1)
                    .map(|p| p.id.clone());
                if let Some(pid2) = pid2 {
                    let (_, token) = TUI_REL_TYPES[state.rel_create_type_idx];
                    let is_child_of = state.rel_create_type_idx == 1; // "Child of" flips order
                    state.mode = InputMode::Normal;
                    state.rel_create_person2_buf.clear();
                    if is_child_of {
                        // "Child of": person1 is child → pid2 is parent (person1)
                        return Action::CreateRelationship(pid2, token.to_string(), pid1);
                    } else {
                        return Action::CreateRelationship(pid1, token.to_string(), pid2);
                    }
                }
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace if state.rel_create_field == 0 => {
            state.rel_create_person2_buf.pop();
        }
        KeyCode::Char(c) if state.rel_create_field == 0 => {
            state.rel_create_person2_buf.push(c);
        }
        _ => {}
    }
    Action::None
}

fn handle_citation_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.citation_event_id = None;
        }
        KeyCode::Tab => {
            state.citation_field = if state.citation_field == 0 { 1 } else { 0 };
        }
        KeyCode::BackTab => {
            state.citation_field = if state.citation_field == 0 { 1 } else { 0 };
        }
        KeyCode::Up if state.citation_field == 0 => {
            state.citation_source_cursor =
                state.citation_source_cursor.saturating_sub(1);
        }
        KeyCode::Down if state.citation_field == 0 => {
            let max = state.citation_source_matches.len().saturating_sub(1);
            if state.citation_source_cursor < max {
                state.citation_source_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(eid) = state.citation_event_id.take() {
                if let Some((sid, _)) =
                    state.citation_source_matches.get(state.citation_source_cursor).cloned()
                {
                    let page = state.citation_page_buf.trim().to_string();
                    state.mode = InputMode::Normal;
                    state.citation_source_buf.clear();
                    state.citation_page_buf.clear();
                    return Action::AddCitation(eid, sid, page);
                }
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace if state.citation_field == 0 => {
            state.citation_source_buf.pop();
            // Refilter
            let q = state.citation_source_buf.to_lowercase();
            state.citation_source_matches = state.sources.iter()
                .filter(|s| s.title.to_lowercase().contains(&q))
                .map(|s| (s.id.clone(), s.title.clone()))
                .collect();
            state.citation_source_cursor = 0;
        }
        KeyCode::Backspace if state.citation_field == 1 => {
            state.citation_page_buf.pop();
        }
        KeyCode::Char(c) if state.citation_field == 0 => {
            state.citation_source_buf.push(c);
            // Refilter
            let q = state.citation_source_buf.to_lowercase();
            state.citation_source_matches = state.sources.iter()
                .filter(|s| s.title.to_lowercase().contains(&q))
                .map(|s| (s.id.clone(), s.title.clone()))
                .collect();
            state.citation_source_cursor = 0;
        }
        KeyCode::Char(c) if state.citation_field == 1 => {
            state.citation_page_buf.push(c);
        }
        _ => {}
    }
    Action::None
}

fn handle_help(_state: &mut TuiState, _key: KeyEvent) -> Action {
    // Any key dismisses the help popup
    _state.mode = InputMode::Normal;
    Action::None
}

fn handle_task_notes_edit(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
            state.task_notes_id = None;
        }
        KeyCode::Enter => {
            if let Some(tid) = state.task_notes_id.take() {
                let notes = state.task_notes_buf.trim().to_string();
                state.mode = InputMode::Normal;
                state.task_notes_buf.clear();
                return Action::UpdateTaskNotes(tid, notes);
            }
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            state.task_notes_buf.pop();
        }
        KeyCode::Char(c) => {
            state.task_notes_buf.push(c);
        }
        _ => {}
    }
    Action::None
}
