use kinforge_app::Application;
use kinforge_core::models::*;
use kinforge_storage::DatabaseStats;

// ── Tab ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    People,
    Tasks,
    Sources,
    Stats,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Tab::People => Tab::Tasks,
            Tab::Tasks => Tab::Sources,
            Tab::Sources => Tab::Stats,
            Tab::Stats => Tab::People,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Tab::People => Tab::Stats,
            Tab::Tasks => Tab::People,
            Tab::Sources => Tab::Tasks,
            Tab::Stats => Tab::Sources,
        }
    }
    pub fn index(self) -> usize {
        match self {
            Tab::People => 0,
            Tab::Tasks => 1,
            Tab::Sources => 2,
            Tab::Stats => 3,
        }
    }
}

// ── InputMode ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    TaskCreate,
    PersonCreate,
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

pub struct SourceRow {
    pub id: SourceId,
    pub title: String,
    pub author: Option<String>,
    pub year: Option<i32>,
    pub citation_count: usize,
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

    // Sources list
    pub sources: Vec<SourceRow>,
    pub sources_selected: usize,
    pub source_detail_open: bool,
    pub source_detail_citations: Vec<(String, String)>, // (event label, page/notes)
    pub source_detail_scroll: usize,

    // Stats
    pub stats: Option<DatabaseStats>,
    pub db_path: String,

    // Task summary counts (kept in sync with tasks vec)
    pub tasks_pending: usize,
    pub tasks_in_progress: usize,
    pub tasks_done: usize,

    // Global
    pub mode: InputMode,
    pub should_quit: bool,

    // Inline task creation
    pub task_create_buf: String,

    // Inline person creation
    pub person_create_given: String,
    pub person_create_surname: String,
    pub person_create_field: u8, // 0 = given, 1 = surname
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
        let tasks_pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        let tasks_in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let tasks_done = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();

        // Load sources with citation counts
        let sources = load_sources(app);

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
            sources,
            sources_selected: 0,
            source_detail_open: false,
            source_detail_citations: vec![],
            source_detail_scroll: 0,
            stats,
            db_path,
            tasks_pending,
            tasks_in_progress,
            tasks_done,
            mode: InputMode::Normal,
            should_quit: false,
            task_create_buf: String::new(),
            person_create_given: String::new(),
            person_create_surname: String::new(),
            person_create_field: 0,
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

    pub fn selected_source(&self) -> Option<&SourceRow> {
        self.sources.get(self.sources_selected)
    }

    /// Reload tasks from the DB and rebuild the row list.
    pub fn reload_tasks(&mut self, app: &Application) {
        self.tasks = app.list_tasks().unwrap_or_default();
        self.task_rows = build_task_rows(&self.tasks);
        self.tasks_pending = self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
        self.tasks_in_progress = self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        self.tasks_done = self.tasks.iter().filter(|t| t.status == TaskStatus::Done).count();
        // Clamp cursor
        let max_item = self
            .task_rows
            .iter()
            .rposition(|r| matches!(r, TaskRow::Item(_)))
            .unwrap_or(0);
        if self.tasks_selected > max_item {
            self.tasks_selected = max_item;
        }
        // Skip to next selectable row if on a header
        if matches!(self.task_rows.get(self.tasks_selected), Some(TaskRow::Header(_))) {
            self.tasks_selected = first_task_item_idx(&self.task_rows);
        }
    }

    /// Reload the people list from the DB and recompute the filter.
    pub fn reload_people(&mut self, app: &Application) {
        use chrono::Datelike;
        let raw_people = app.list_people().unwrap_or_default();
        self.people.clear();
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
            self.people.push(PersonRow { id: p.id, display_name, birth_year });
        }
        self.recompute_filter();
        // Select the last person (just added)
        if !self.filtered_people.is_empty() {
            self.people_selected = self.filtered_people.len() - 1;
        }
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

pub fn first_task_item_idx(rows: &[TaskRow]) -> usize {
    rows.iter()
        .position(|r| matches!(r, TaskRow::Item(_)))
        .unwrap_or(0)
}

fn load_sources(app: &Application) -> Vec<SourceRow> {
    let raw = app.list_sources().unwrap_or_default();
    raw.into_iter()
        .map(|s| {
            let citation_count = app
                .list_citations_for_source(&s.id)
                .map(|v| v.len())
                .unwrap_or(0);
            SourceRow {
                id: s.id,
                title: s.title,
                author: s.author,
                year: s.year,
                citation_count,
            }
        })
        .collect()
}
