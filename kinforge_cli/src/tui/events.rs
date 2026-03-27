use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use kinforge_core::models::{PersonId, SourceId, TaskId};

use super::state::{InputMode, Tab, TaskRow, TuiState};

pub enum Action {
    None,
    Quit,
    OpenPersonDetail(PersonId),
    OpenSourceDetail(SourceId),
    CompleteTask(TaskId),
    CreateTask(String),
    CycleTaskPriority(TaskId),
    DeleteTask(TaskId),
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
