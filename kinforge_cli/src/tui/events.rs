use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use kinforge_core::models::{PersonId, SourceId, TaskId};

use super::state::{
    build_filtered_task_rows, first_task_item_idx, InputMode, Tab, TaskRow,
    TuiState, TUI_EVENT_TYPES,
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
    CreatePerson(String, String),   // (given, surname)
    EditPerson(PersonId, String, String), // (id, given, surname)
    CreateSource(String, String),  // (title, author)
    DeletePerson(PersonId),
    CreateEvent(PersonId, String, String, String), // (person_id, type_name, date_str, place_str)
    DeleteSource(SourceId),
    EditTask(TaskId, String, kinforge_core::models::TaskPriority),
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
            Action::None
        }
        KeyCode::BackTab => {
            state.active_tab = state.active_tab.prev();
            state.detail_open = false;
            state.source_detail_open = false;
            Action::None
        }

        KeyCode::Up | KeyCode::Char('k') => {
            match state.active_tab {
                Tab::People => {
                    if state.detail_open {
                        state.detail_scroll = state.detail_scroll.saturating_sub(1);
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
                        state.detail_scroll = state.detail_scroll.saturating_add(1);
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

        KeyCode::Esc => {
            if state.detail_open {
                state.detail_open = false;
                state.detail_scroll = 0;
            } else if state.source_detail_open {
                state.source_detail_open = false;
                state.source_detail_scroll = 0;
            }
            Action::None
        }

        _ => Action::None,
    }
}

fn handle_search(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.search_active = false;
            state.search_query.clear();
            state.mode = InputMode::Normal;
            state.recompute_filter();
        }
        KeyCode::Enter => {
            state.search_active = false;
            state.mode = InputMode::Normal;
            if let Some(row) = state.selected_person() {
                return Action::OpenPersonDetail(row.id.clone());
            }
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            state.recompute_filter();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.people_selected > 0 {
                state.people_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let max = state.filtered_people.len().saturating_sub(1);
            if state.people_selected < max {
                state.people_selected += 1;
            }
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            state.recompute_filter();
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

fn handle_person_create(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.mode = InputMode::Normal;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            state.person_create_field = 1 - state.person_create_field;
        }
        KeyCode::Enter => {
            let given = state.person_create_given.trim().to_string();
            let surname = state.person_create_surname.trim().to_string();
            state.mode = InputMode::Normal;
            if !given.is_empty() || !surname.is_empty() {
                return Action::CreatePerson(given, surname);
            }
        }
        KeyCode::Backspace => {
            if state.person_create_field == 0 {
                state.person_create_given.pop();
            } else {
                state.person_create_surname.pop();
            }
        }
        KeyCode::Char(c) => {
            if state.person_create_field == 0 {
                state.person_create_given.push(c);
            } else {
                state.person_create_surname.push(c);
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
        }
        _ => {
            state.confirm_person_id = None;
            state.confirm_source_id = None;
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
