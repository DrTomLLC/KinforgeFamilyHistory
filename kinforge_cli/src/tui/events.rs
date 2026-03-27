use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use kinforge_core::models::PersonId;

use super::state::{InputMode, Tab, TaskRow, TuiState};

pub enum Action {
    None,
    Quit,
    OpenDetail(PersonId),
}

pub fn handle_key(state: &mut TuiState, key: KeyEvent) -> Action {
    // Only react to key-press events (not release/repeat on some platforms)
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
        InputMode::Detail => handle_detail(state, key),
    }
}

fn handle_normal(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') => Action::Quit,

        KeyCode::Tab => {
            state.active_tab = state.active_tab.next();
            Action::None
        }
        KeyCode::BackTab => {
            state.active_tab = state.active_tab.prev();
            Action::None
        }

        KeyCode::Up | KeyCode::Char('k') => {
            match state.active_tab {
                Tab::People => {
                    if state.people_selected > 0 {
                        state.people_selected -= 1;
                    }
                }
                Tab::Tasks => move_task_up(state),
                Tab::Stats => {}
            }
            Action::None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            match state.active_tab {
                Tab::People => {
                    let max = state.filtered_people.len().saturating_sub(1);
                    if state.people_selected < max {
                        state.people_selected += 1;
                    }
                }
                Tab::Tasks => move_task_down(state),
                Tab::Stats => {}
            }
            Action::None
        }

        KeyCode::Char('/') if state.active_tab == Tab::People => {
            state.search_active = true;
            state.mode = InputMode::Search;
            Action::None
        }

        KeyCode::Enter if state.active_tab == Tab::People => {
            if let Some(row) = state.selected_person() {
                Action::OpenDetail(row.id.clone())
            } else {
                Action::None
            }
        }

        KeyCode::Esc if state.detail_open => {
            state.detail_open = false;
            state.detail_scroll = 0;
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
            // Confirm search — stay in Normal, keep filter active
            state.search_active = false;
            state.mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            state.recompute_filter();
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            state.recompute_filter();
        }
        _ => {}
    }
    Action::None
}

fn handle_detail(state: &mut TuiState, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => {
            state.detail_open = false;
            state.mode = InputMode::Normal;
            state.detail_scroll = 0;
        }
        KeyCode::Char('q') => return Action::Quit,
        KeyCode::Up | KeyCode::Char('k') => {
            state.detail_scroll = state.detail_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            state.detail_scroll = state.detail_scroll.saturating_add(1);
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
