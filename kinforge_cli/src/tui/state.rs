use kinforge_app::Application;
use kinforge_core::models::*;
use kinforge_storage::DatabaseStats;

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    People,
    Tasks,
    Stats,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::People => Tab::Tasks,
            Tab::Tasks => Tab::Stats,
            Tab::Stats => Tab::People,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Tab::People => Tab::Stats,
            Tab::Tasks => Tab::People,
            Tab::Stats => Tab::Tasks,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Tab::People => 0,
            Tab::Tasks => 1,
            Tab::Stats => 2,
        }
    }
}

// ── InputMode ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Detail,
}

// ── Row types ─────────────────────────────────────────────────────────────────

pub struct PersonRow {
    pub id: PersonId,
    pub display_name: String,
    pub birth_year: Option<i32>,
}

pub enum TaskRow {
    Header(String),
    Item(usize), // index into TuiState::tasks
}

// ── Main state ────────────────────────────────────────────────────────────────

pub struct TuiState {
    // Tab selection
    pub active_tab: Tab,

    // People list
    pub people: Vec<PersonRow>,
    pub people_selected: usize,

    // Search
    pub search_active: bool,
    pub search_query: String,
    pub filtered_people: Vec<usize>, // indices into `people`

    // Person detail panel
    pub detail_open: bool,
    pub detail_person_id: Option<PersonId>,
    pub detail_events: Vec<Event>,
    pub detail_rel_rows: Vec<(String, String)>, // (label, other name)
    pub detail_scroll: usize,

    // Tasks list
    pub tasks: Vec<Task>,
    pub task_rows: Vec<TaskRow>,
    pub tasks_selected: usize,

    // Stats
    pub stats: Option<DatabaseStats>,
    pub db_path: String,

    // Global
    pub mode: InputMode,
    pub should_quit: bool,
}

impl TuiState {
    pub fn new(app: &Application) -> anyhow::Result<Self> {
        use chrono::Datelike;

        // Load people with birth years
        let raw_people = app.list_people()?;
        let mut people = Vec::with_capacity(raw_people.len());
        for p in raw_people {
            let events = app.list_events_for_person(&p.id).unwrap_or_default();
            let birth_year = events
                .iter()
                .find(|e| matches!(e.event_type, EventType::Birth))
                .and_then(|e| e.date.as_ref())
                .and_then(|d| match d {
                    EventDate::Exact(nd) | EventDate::Approximate(nd) => Some(nd.year()),
                    _ => None,
                });
            let display_name = p.display_name();
            people.push(PersonRow {
                id: p.id,
                display_name,
                birth_year,
            });
        }

        let filtered_people: Vec<usize> = (0..people.len()).collect();

        // Load tasks
        let tasks = app.list_tasks().unwrap_or_default();
        let task_rows = build_task_rows(&tasks);
        let first_task = first_task_item_idx(&task_rows);

        // Stats
        let stats = app.stats().ok();
        let db_path = app.config.database_path.display().to_string();

        Ok(Self {
            active_tab: Tab::People,
            people,
            people_selected: 0,
            search_active: false,
            search_query: String::new(),
            filtered_people,
            detail_open: false,
            detail_person_id: None,
            detail_events: vec![],
            detail_rel_rows: vec![],
            detail_scroll: 0,
            tasks,
            task_rows,
            tasks_selected: first_task,
            stats,
            db_path,
            mode: InputMode::Normal,
            should_quit: false,
        })
    }

    pub fn recompute_filter(&mut self) {
        let q = self.search_query.to_lowercase();
        self.filtered_people = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.display_name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        if self.people_selected >= self.filtered_people.len() {
            self.people_selected = self.filtered_people.len().saturating_sub(1);
        }
    }

    pub fn selected_person(&self) -> Option<&PersonRow> {
        let idx = *self.filtered_people.get(self.people_selected)?;
        self.people.get(idx)
    }
}

pub fn build_task_rows(tasks: &[Task]) -> Vec<TaskRow> {
    let mut rows = Vec::new();

    let in_progress: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == TaskStatus::InProgress)
        .map(|(i, _)| i)
        .collect();
    let pending: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == TaskStatus::Pending)
        .map(|(i, _)| i)
        .collect();
    let done: Vec<usize> = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == TaskStatus::Done)
        .map(|(i, _)| i)
        .collect();

    if !in_progress.is_empty() {
        rows.push(TaskRow::Header("IN PROGRESS".to_string()));
        for i in in_progress {
            rows.push(TaskRow::Item(i));
        }
    }
    if !pending.is_empty() {
        rows.push(TaskRow::Header("PENDING".to_string()));
        for i in pending {
            rows.push(TaskRow::Item(i));
        }
    }
    if !done.is_empty() {
        rows.push(TaskRow::Header("DONE".to_string()));
        for i in done {
            rows.push(TaskRow::Item(i));
        }
    }

    rows
}

fn first_task_item_idx(rows: &[TaskRow]) -> usize {
    rows.iter()
        .position(|r| matches!(r, TaskRow::Item(_)))
        .unwrap_or(0)
}
