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

/// Ordered list of event types available in the TUI add-event popup.
pub const TUI_EVENT_TYPES: &[&str] = &[
    "Birth", "Death", "Marriage", "Divorce", "Baptism", "Burial",
    "Residence", "Occupation", "Census", "Military",
    "Emigration", "Immigration", "Naturalization", "Education",
];

/// Relationship types (display label, parse token) for the TUI add-relationship popup.
pub const TUI_REL_TYPES: &[(&str, &str)] = &[
    ("Parent of", "parent"),
    ("Child of", "parent"),   // will flip person order
    ("Spouse", "spouse"),
    ("Sibling", "sibling"),
    ("Half-sibling", "halfsibling"),
    ("Adoptive parent", "adoptive"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    TaskCreate,
    TaskEdit,
    PersonCreate,
    PersonEdit,
    PersonNotesEdit,
    SourceCreate,
    SourceEdit,
    ConfirmDelete,
    EventCreate,
    EventEdit,
    RelationshipCreate,
}

// ── SortOrder ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Name,
    BirthYear,
}

impl SortOrder {
    pub fn next(self) -> Self {
        match self {
            SortOrder::Name => SortOrder::BirthYear,
            SortOrder::BirthYear => SortOrder::Name,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Name => "name",
            SortOrder::BirthYear => "birth yr",
        }
    }
}

// ── TaskStatusFilter ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskStatusFilter {
    All,
    Pending,
    InProgress,
    Done,
}

impl TaskStatusFilter {
    pub fn next(self) -> Self {
        match self {
            TaskStatusFilter::All => TaskStatusFilter::Pending,
            TaskStatusFilter::Pending => TaskStatusFilter::InProgress,
            TaskStatusFilter::InProgress => TaskStatusFilter::Done,
            TaskStatusFilter::Done => TaskStatusFilter::All,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            TaskStatusFilter::All => "all",
            TaskStatusFilter::Pending => "pending",
            TaskStatusFilter::InProgress => "in progress",
            TaskStatusFilter::Done => "done",
        }
    }
}

// ── Row types ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
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
    pub detail_event_places: Vec<Option<String>>, // parallel to detail_events
    pub detail_rel_rows: Vec<(String, String)>, // (label, other name)
    pub detail_scroll: usize,
    pub detail_notes: Option<String>,
    pub detail_media_count: usize,
    pub detail_event_cursor: usize, // which event is "selected" in the detail panel

    // Tasks list
    pub tasks: Vec<Task>,
    pub task_rows: Vec<TaskRow>,
    pub tasks_selected: usize,
    pub task_status_filter: TaskStatusFilter,

    // Sources list
    pub sources: Vec<SourceRow>,
    pub sources_selected: usize,
    pub filtered_sources: Vec<usize>, // indices into `sources`
    pub source_search_query: String,
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

    // People sort order
    pub sort_order: SortOrder,

    // Delete confirmation (person or source)
    pub confirm_name: String,
    pub confirm_person_id: Option<PersonId>,
    pub confirm_source_id: Option<SourceId>,

    // Top places by event count (for Stats tab)
    pub top_places: Vec<(String, usize)>, // (place name, event count)

    // Inline task creation
    pub task_create_buf: String,

    // Inline task edit
    pub task_edit_desc: String,
    pub task_edit_priority_idx: usize, // 0=Low, 1=Medium, 2=High
    pub task_edit_id: Option<TaskId>,
    pub task_edit_field: u8, // 0 = description, 1 = priority

    // Inline person creation
    pub person_create_given: String,
    pub person_create_surname: String,
    pub person_create_sex: u8,   // 0 = Unknown, 1 = Male, 2 = Female
    pub person_create_field: u8, // 0 = given, 1 = surname, 2 = sex

    // Inline person edit (edit primary name of selected person)
    pub person_edit_given: String,
    pub person_edit_surname: String,
    pub person_edit_field: u8, // 0 = given, 1 = surname
    pub person_edit_id: Option<PersonId>,

    // Inline source creation
    pub source_create_title: String,
    pub source_create_author: String,
    pub source_create_field: u8, // 0 = title, 1 = author

    // Inline source edit
    pub source_edit_id: Option<SourceId>,
    pub source_edit_title: String,
    pub source_edit_author: String,
    pub source_edit_year: String,    // stored as string for typing; parsed on Enter
    pub source_edit_field: u8,       // 0 = title, 1 = author, 2 = year

    // Inline event creation (from person detail panel)
    pub event_create_type_idx: usize,   // index into TUI_EVENT_TYPES
    pub event_create_date: String,      // YYYY-MM-DD or empty
    pub event_create_place: String,
    pub event_create_field: u8,         // 0 = type, 1 = date, 2 = place
    pub event_create_person_id: Option<PersonId>,

    // Inline event edit (from person detail panel)
    pub event_edit_id: Option<EventId>,
    pub event_edit_type_idx: usize,
    pub event_edit_date: String,
    pub event_edit_place: String,
    pub event_edit_field: u8,           // 0 = type, 1 = date, 2 = place

    // Delete confirmation for events (separate from person/source confirm)
    pub confirm_event_id: Option<EventId>,

    // Inline relationship creation (from person detail panel)
    pub rel_create_person2_buf: String, // name fragment to match against people list
    pub rel_create_type_idx: usize,     // index into TUI_REL_TYPES
    pub rel_create_field: u8,           // 0 = person2 name, 1 = rel type
    pub rel_create_person1_id: Option<PersonId>,

    // Inline person notes edit
    pub person_notes_buf: String,
    pub person_notes_id: Option<PersonId>,

    // Task detail panel
    pub task_detail_open: bool,
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
        let task_rows = build_filtered_task_rows(&tasks, TaskStatusFilter::All);
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
            detail_event_places: vec![],
            detail_rel_rows: vec![],
            detail_scroll: 0,
            detail_notes: None,
            detail_media_count: 0,
            detail_event_cursor: 0,
            tasks,
            task_rows,
            tasks_selected: first_task,
            task_status_filter: TaskStatusFilter::All,
            filtered_sources: (0..sources.len()).collect(),
            source_search_query: String::new(),
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
            sort_order: SortOrder::Name,
            confirm_name: String::new(),
            confirm_person_id: None,
            confirm_source_id: None,
            top_places: load_top_places(app),
            task_create_buf: String::new(),
            task_edit_desc: String::new(),
            task_edit_priority_idx: 1,
            task_edit_id: None,
            task_edit_field: 0,
            person_create_given: String::new(),
            person_create_surname: String::new(),
            person_create_sex: 0,
            person_create_field: 0,
            person_edit_given: String::new(),
            person_edit_surname: String::new(),
            person_edit_field: 0,
            person_edit_id: None,
            source_create_title: String::new(),
            source_create_author: String::new(),
            source_create_field: 0,
            source_edit_id: None,
            source_edit_title: String::new(),
            source_edit_author: String::new(),
            source_edit_year: String::new(),
            source_edit_field: 0,
            event_create_type_idx: 0,
            event_create_date: String::new(),
            event_create_place: String::new(),
            event_create_field: 0,
            event_create_person_id: None,
            event_edit_id: None,
            event_edit_type_idx: 0,
            event_edit_date: String::new(),
            event_edit_place: String::new(),
            event_edit_field: 0,
            confirm_event_id: None,
            rel_create_person2_buf: String::new(),
            rel_create_type_idx: 0,
            rel_create_field: 0,
            rel_create_person1_id: None,
            person_notes_buf: String::new(),
            person_notes_id: None,
            task_detail_open: false,
        })
    }

    pub fn recompute_filter(&mut self) {
        let q = self.search_query.to_lowercase();
        let mut indices: Vec<usize> = self
            .people
            .iter()
            .enumerate()
            .filter(|(_, p)| p.display_name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        // Apply sort
        match self.sort_order {
            SortOrder::Name => {
                indices.sort_by(|&a, &b| {
                    self.people[a].display_name.cmp(&self.people[b].display_name)
                });
            }
            SortOrder::BirthYear => {
                indices.sort_by(|&a, &b| {
                    self.people[a].birth_year.cmp(&self.people[b].birth_year)
                });
            }
        }
        self.filtered_people = indices;
        if self.people_selected >= self.filtered_people.len() {
            self.people_selected = self.filtered_people.len().saturating_sub(1);
        }
    }

    pub fn selected_person(&self) -> Option<&PersonRow> {
        let idx = *self.filtered_people.get(self.people_selected)?;
        self.people.get(idx)
    }

    /// Reload tasks from the DB and rebuild the row list.
    pub fn reload_tasks(&mut self, app: &Application) {
        self.tasks = app.list_tasks().unwrap_or_default();
        self.task_rows = build_filtered_task_rows(&self.tasks, self.task_status_filter);
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

    /// Reload the sources list from the DB and select the last entry.
    pub fn reload_sources(&mut self, app: &Application) {
        self.sources = load_sources(app);
        self.recompute_source_filter();
        if !self.filtered_sources.is_empty() {
            self.sources_selected = self.filtered_sources.len() - 1;
        }
    }

    pub fn recompute_source_filter(&mut self) {
        let q = self.source_search_query.to_lowercase();
        self.filtered_sources = self.sources
            .iter()
            .enumerate()
            .filter(|(_, s)| s.title.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        if self.sources_selected >= self.filtered_sources.len() {
            self.sources_selected = self.filtered_sources.len().saturating_sub(1);
        }
    }

    pub fn selected_source(&self) -> Option<&SourceRow> {
        let idx = *self.filtered_sources.get(self.sources_selected)?;
        self.sources.get(idx)
    }

    /// Recompute top-places (called after events change).
    pub fn reload_top_places(&mut self, app: &Application) {
        self.top_places = load_top_places(app);
    }
}

/// Build task rows respecting the active status filter.
pub fn build_filtered_task_rows(tasks: &[Task], filter: TaskStatusFilter) -> Vec<TaskRow> {
    let filtered: Vec<(usize, &Task)> = tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| match filter {
            TaskStatusFilter::All => true,
            TaskStatusFilter::Pending => t.status == TaskStatus::Pending,
            TaskStatusFilter::InProgress => t.status == TaskStatus::InProgress,
            TaskStatusFilter::Done => t.status == TaskStatus::Done,
        })
        .collect();

    if filter != TaskStatusFilter::All {
        // Flat list when filtering — no section headers
        return filtered
            .into_iter()
            .map(|(idx, _)| TaskRow::Item(idx))
            .collect();
    }

    // Full grouped display
    build_task_rows(tasks)
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

fn load_top_places(app: &Application) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let events = app.database().list_all_events().unwrap_or_default();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in &events {
        if let Some(ref pid) = e.place_id {
            if let Ok(place) = app.get_place(pid) {
                *counts.entry(place.name).or_insert(0) += 1;
            }
        }
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted.truncate(5);
    sorted
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
