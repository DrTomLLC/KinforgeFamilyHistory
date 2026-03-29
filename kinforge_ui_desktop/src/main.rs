#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use kinforge_app::{Application, NotesMatch};
use kinforge_config::Config;
use kinforge_core::models::*;
use kinforge_import_export::{export_gedcom, import_gedcom};
use kinforge_reports::{
    ancestor_report, descendant_report, family_group_sheet, individual_report, people_list_report,
};
use std::path::PathBuf;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Kinforge Family History"),
        ..Default::default()
    };
    eframe::run_native(
        "Kinforge Family History",
        options,
        Box::new(|_cc| Ok(Box::new(KinforgeApp::new()))),
    )
}

// ── Tabs ───────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    People,
    Sources,
    Reports,
    ImportExport,
    Settings,
}

// ── Forms ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct PersonForm {
    given: String,
    surname: String,
    sex: String,
    notes: String,
}

#[derive(Default)]
struct EventForm {
    event_type: String,
    date_str: String,
    place: String,
    notes: String,
}

#[derive(Default)]
struct SourceForm {
    title: String,
    author: String,
    publication: String,
    year_str: String,
    repository: String,
    notes: String,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct KinforgeApp {
    app: Option<Application>,
    open_error: Option<String>,
    db_path_str: String,

    current_tab: Tab,

    // People
    people: Vec<Person>,
    people_search: String,
    selected_person_id: Option<PersonId>,
    person_events: Vec<Event>,
    person_relationships: Vec<Relationship>,
    show_add_person: bool,
    person_form: PersonForm,
    show_add_event: bool,
    event_form: EventForm,
    show_add_relationship: bool,
    rel_person2_id_str: String,
    rel_type_str: String,
    rel_notes: String,
    status_msg: String,

    // Sources
    sources: Vec<Source>,
    sources_search: String,
    show_add_source: bool,
    source_form: SourceForm,

    // Reports
    report_output: String,
    report_person_id_str: String,
    report_generations_str: String,

    // Notes search
    notes_search_query: String,
    notes_search_results: Vec<NotesMatch>,

    // Import / Export
    import_path_str: String,
    export_path_str: String,
    ie_status: String,
}

impl KinforgeApp {
    fn new() -> Self {
        Self {
            app: None,
            open_error: None,
            db_path_str: "kinforge.db".to_string(),
            current_tab: Tab::People,
            people: Vec::new(),
            people_search: String::new(),
            selected_person_id: None,
            person_events: Vec::new(),
            person_relationships: Vec::new(),
            show_add_person: false,
            person_form: PersonForm::default(),
            show_add_event: false,
            event_form: EventForm::default(),
            show_add_relationship: false,
            rel_person2_id_str: String::new(),
            rel_type_str: "ParentChild".to_string(),
            rel_notes: String::new(),
            status_msg: String::new(),
            sources: Vec::new(),
            sources_search: String::new(),
            show_add_source: false,
            source_form: SourceForm::default(),
            report_output: String::new(),
            report_person_id_str: String::new(),
            report_generations_str: "4".to_string(),
            notes_search_query: String::new(),
            notes_search_results: Vec::new(),
            import_path_str: String::new(),
            export_path_str: String::new(),
            ie_status: String::new(),
        }
    }

    fn open_db(&mut self) {
        let path = PathBuf::from(self.db_path_str.trim());
        let config = Config {
            database_path: path,
            ..Default::default()
        };
        match Application::open(config) {
            Ok(a) => {
                self.open_error = None;
                self.app = Some(a);
                self.refresh_people();
                self.refresh_sources();
                self.status_msg = "Database opened.".to_string();
            }
            Err(e) => {
                self.open_error = Some(e.to_string());
            }
        }
    }

    fn refresh_people(&mut self) {
        if let Some(app) = &self.app {
            self.people = app.list_people().unwrap_or_default();
        }
    }

    fn refresh_sources(&mut self) {
        if let Some(app) = &self.app {
            self.sources = app.list_sources().unwrap_or_default();
        }
    }

    fn refresh_selected_person(&mut self) {
        if let (Some(app), Some(pid)) = (&self.app, &self.selected_person_id) {
            self.person_events = app.list_events_for_person(pid).unwrap_or_default();
            self.person_relationships = app.list_relationships_for_person(pid).unwrap_or_default();
        }
    }
}

// ── eframe::App ───────────────────────────────────────────────────────────────

impl eframe::App for KinforgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Show DB opener screen if no database is open
        if self.app.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(160.0);
                    ui.label(
                        egui::RichText::new("Kinforge Family History")
                            .size(30.0)
                            .color(egui::Color32::from_rgb(80, 140, 200)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Local-first genealogy software")
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(24.0);
                    ui.label("Database file:");
                    ui.add_sized(
                        [320.0, 24.0],
                        egui::TextEdit::singleline(&mut self.db_path_str),
                    );
                    ui.add_space(8.0);
                    if ui.button("  Open / Create Database  ").clicked() {
                        self.open_db();
                    }
                    if let Some(err) = &self.open_error.clone() {
                        ui.add_space(8.0);
                        ui.colored_label(egui::Color32::RED, err);
                    }
                });
            });
            return;
        }

        // Tab bar
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::People, "👥  People");
                ui.selectable_value(&mut self.current_tab, Tab::Sources, "📚  Sources");
                ui.selectable_value(&mut self.current_tab, Tab::Reports, "📊  Reports");
                ui.selectable_value(&mut self.current_tab, Tab::ImportExport, "📂  Import / Export");
                ui.selectable_value(&mut self.current_tab, Tab::Settings, "⚙  Settings");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status_msg.is_empty() {
                        ui.label(
                            egui::RichText::new(&self.status_msg)
                                .small()
                                .color(egui::Color32::from_rgb(140, 200, 140)),
                        );
                    }
                });
            });
        });

        match self.current_tab {
            Tab::People => self.tab_people(ctx),
            Tab::Sources => self.tab_sources(ctx),
            Tab::Reports => self.tab_reports(ctx),
            Tab::ImportExport => self.tab_import_export(ctx),
            Tab::Settings => self.tab_settings(ctx),
        }
    }
}

// ── People tab ────────────────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_people(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("people_list")
            .resizable(true)
            .default_width(280.0)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("People");
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.people_search)
                            .hint_text("Search…")
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(4.0);
                if ui.button("➕  Add Person").clicked() {
                    self.show_add_person = true;
                    self.person_form = PersonForm::default();
                }
                ui.separator();

                let search_lower = self.people_search.to_lowercase();
                let total = self.people.len();
                let filtered: Vec<Person> = self
                    .people
                    .iter()
                    .filter(|p| {
                        search_lower.is_empty()
                            || p.display_name().to_lowercase().contains(&search_lower)
                    })
                    .cloned()
                    .collect();

                ui.label(
                    egui::RichText::new(format!("{} / {} people", filtered.len(), total))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(2.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for person in &filtered {
                            let name = person.display_name();
                            let selected =
                                self.selected_person_id.as_ref() == Some(&person.id);
                            let sex_col = sex_color(&person.sex);
                            let label = egui::RichText::new(&name).color(if selected {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::LIGHT_GRAY
                            });
                            let resp = ui.selectable_label(selected, label);
                            if resp.clicked() {
                                self.selected_person_id = Some(person.id.clone());
                                self.refresh_selected_person();
                                self.status_msg = format!("Selected: {}", name);
                            }
                            // sex dot
                            let dot = egui::pos2(resp.rect.right() - 10.0, resp.rect.center().y);
                            ui.painter().circle_filled(dot, 4.0, sex_col);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let selected_id = self.selected_person_id.clone();
            if let Some(pid) = &selected_id {
                let person_opt = self.people.iter().find(|p| &p.id == pid).cloned();
                if let Some(person) = person_opt {
                    ui.horizontal(|ui| {
                        ui.heading(person.display_name());
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(sex_label(&person.sex))
                                .color(sex_color(&person.sex))
                                .strong(),
                        );
                    });
                    ui.label(
                        egui::RichText::new(format!("ID: {}", person.id))
                            .small()
                            .color(egui::Color32::GRAY),
                    );

                    if let Some(n) = &person.notes {
                        if !n.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("📝 {}", n))
                                    .italics()
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            );
                        }
                    }

                    ui.separator();

                    // Names
                    if !person.names.is_empty() {
                        ui.label(egui::RichText::new("Names").strong());
                        for name in &person.names {
                            let g = name.given.as_deref().unwrap_or("");
                            let s = name.surname.as_deref().unwrap_or("");
                            ui.label(format!("  {} {} ({:?})", g, s, name.name_type));
                        }
                        ui.add_space(4.0);
                    }

                    // Events
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Events ({})", self.person_events.len())).strong());
                        if ui.small_button("➕ Add").clicked() {
                            self.show_add_event = true;
                            self.event_form = EventForm::default();
                        }
                    });
                    egui::Grid::new("events_grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Type").strong());
                            ui.label(egui::RichText::new("Date").strong());
                            ui.label(egui::RichText::new("Notes").strong());
                            ui.end_row();
                            for ev in &self.person_events.clone() {
                                let type_col = event_type_color(&ev.event_type);
                                ui.label(
                                    egui::RichText::new(ev.event_type.to_string())
                                        .color(type_col),
                                );
                                let date_str = ev
                                    .date
                                    .as_ref()
                                    .map(|d| d.to_string())
                                    .unwrap_or_else(|| "—".to_string());
                                ui.label(&date_str);
                                ui.label(
                                    egui::RichText::new(ev.notes.as_deref().unwrap_or(""))
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                                ui.end_row();
                            }
                        });

                    ui.add_space(8.0);

                    // Relationships
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Relationships ({})", self.person_relationships.len())).strong());
                        if ui.small_button("➕ Add").clicked() {
                            self.show_add_relationship = true;
                        }
                    });
                    egui::Grid::new("rels_grid")
                        .num_columns(2)
                        .striped(true)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for rel in &self.person_relationships.clone() {
                                ui.label(
                                    egui::RichText::new(rel.rel_type.to_string())
                                        .color(egui::Color32::from_rgb(180, 140, 220)),
                                );
                                let other_id = if rel.person1_id == *pid {
                                    &rel.person2_id
                                } else {
                                    &rel.person1_id
                                };
                                let other_name = self
                                    .app
                                    .as_ref()
                                    .and_then(|a| a.get_person(other_id).ok())
                                    .map(|p| p.display_name())
                                    .unwrap_or_else(|| other_id.to_string());
                                ui.label(&other_name);
                                ui.end_row();
                            }
                        });

                    ui.add_space(12.0);
                    ui.separator();
                    if ui
                        .button(egui::RichText::new("🗑  Delete Person").color(egui::Color32::RED))
                        .clicked()
                    {
                        if let Some(app) = &self.app {
                            match app.delete_person(pid) {
                                Ok(_) => self.status_msg = "Person deleted.".to_string(),
                                Err(e) => self.status_msg = e.to_string(),
                            }
                        }
                        self.selected_person_id = None;
                        self.person_events.clear();
                        self.person_relationships.clear();
                        self.refresh_people();
                    }
                } else {
                    self.selected_person_id = None;
                }
            } else {
                ui.add_space(100.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Select a person from the list, or add a new one.")
                            .color(egui::Color32::GRAY),
                    );
                });
            }
        });

        // ── Add Person window ──────────────────────────────────────────────────
        if self.show_add_person {
            let mut open = true;
            egui::Window::new("Add Person")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("add_person_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Given name:");
                            ui.text_edit_singleline(&mut self.person_form.given);
                            ui.end_row();
                            ui.label("Surname:");
                            ui.text_edit_singleline(&mut self.person_form.surname);
                            ui.end_row();
                            ui.label("Sex:");
                            ui.text_edit_singleline(&mut self.person_form.sex);
                            ui.end_row();
                            ui.label("Notes:");
                            ui.text_edit_multiline(&mut self.person_form.notes);
                            ui.end_row();
                        });
                    ui.label(
                        egui::RichText::new("Sex: Male / Female / Unknown")
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(4.0);
                    if ui.button("Save").clicked() {
                        let sex: Sex =
                            self.person_form.sex.parse().unwrap_or(Sex::Unknown);
                        let given = opt_str(&self.person_form.given);
                        let surname = opt_str(&self.person_form.surname);
                        let notes = opt_str(&self.person_form.notes);
                        let mut saved = false;
                        if let Some(app) = &self.app {
                            match app.add_person(given, surname, sex, notes) {
                                Ok(_) => {
                                    self.status_msg = "Person added.".to_string();
                                    saved = true;
                                }
                                Err(e) => self.status_msg = e.to_string(),
                            }
                        }
                        if saved {
                            self.show_add_person = false;
                            self.refresh_people();
                        }
                    }
                });
            if !open {
                self.show_add_person = false;
            }
        }

        // ── Add Event window ───────────────────────────────────────────────────
        if self.show_add_event {
            if let Some(pid) = self.selected_person_id.clone() {
                let mut open = true;
                egui::Window::new("Add Event")
                    .open(&mut open)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("add_event_grid")
                            .num_columns(2)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                ui.label("Type:");
                                ui.text_edit_singleline(&mut self.event_form.event_type);
                                ui.end_row();
                                ui.label("Date:");
                                ui.text_edit_singleline(&mut self.event_form.date_str);
                                ui.end_row();
                                ui.label("Place:");
                                ui.text_edit_singleline(&mut self.event_form.place);
                                ui.end_row();
                                ui.label("Notes:");
                                ui.text_edit_multiline(&mut self.event_form.notes);
                                ui.end_row();
                            });
                        ui.label(
                            egui::RichText::new(
                                "Type: Birth / Death / Marriage / Census / Residence / …\n\
                                 Date: YYYY or YYYY-MM-DD",
                            )
                            .small()
                            .color(egui::Color32::GRAY),
                        );
                        ui.add_space(4.0);
                        if ui.button("Save").clicked() {
                            let et = parse_event_type(&self.event_form.event_type);
                            let date = parse_date(&self.event_form.date_str);
                            let place = opt_str(&self.event_form.place);
                            let notes = opt_str(&self.event_form.notes);
                            let mut saved = false;
                            if let Some(app) = &self.app {
                                match app.add_event(pid.clone(), et, date, place, notes) {
                                    Ok(_) => {
                                        self.status_msg = "Event added.".to_string();
                                        saved = true;
                                    }
                                    Err(e) => self.status_msg = e.to_string(),
                                }
                            }
                            if saved {
                                self.show_add_event = false;
                                self.refresh_selected_person();
                            }
                        }
                    });
                if !open {
                    self.show_add_event = false;
                }
            }
        }

        // ── Add Relationship window ────────────────────────────────────────────
        if self.show_add_relationship {
            if let Some(pid) = self.selected_person_id.clone() {
                let mut open = true;
                egui::Window::new("Add Relationship")
                    .open(&mut open)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("add_rel_grid")
                            .num_columns(2)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                ui.label("Other person ID:");
                                ui.text_edit_singleline(&mut self.rel_person2_id_str);
                                ui.end_row();
                                ui.label("Type:");
                                ui.text_edit_singleline(&mut self.rel_type_str);
                                ui.end_row();
                                ui.label("Notes:");
                                ui.text_edit_multiline(&mut self.rel_notes);
                                ui.end_row();
                            });
                        ui.label(
                            egui::RichText::new("Type: ParentChild / Spouse / Sibling")
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        ui.add_space(4.0);
                        if ui.button("Save").clicked() {
                            let rt = parse_rel_type(&self.rel_type_str);
                            let p2_result =
                                PersonId::from_str(self.rel_person2_id_str.trim());
                            match p2_result {
                                Err(_) => {
                                    self.status_msg =
                                        "Invalid person ID (must be a UUID).".to_string()
                                }
                                Ok(p2) => {
                                    let notes = opt_str(&self.rel_notes);
                                    let mut saved = false;
                                    if let Some(app) = &self.app {
                                        match app.add_relationship(rt, pid.clone(), p2, notes) {
                                            Ok(_) => {
                                                self.status_msg =
                                                    "Relationship added.".to_string();
                                                saved = true;
                                            }
                                            Err(e) => self.status_msg = e.to_string(),
                                        }
                                    }
                                    if saved {
                                        self.show_add_relationship = false;
                                        self.refresh_selected_person();
                                    }
                                }
                            }
                        }
                    });
                if !open {
                    self.show_add_relationship = false;
                }
            }
        }
    }
}

// ── Sources tab ───────────────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_sources(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sources_list")
            .resizable(true)
            .default_width(300.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Sources");
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sources_search)
                            .hint_text("Search…")
                            .desired_width(f32::INFINITY),
                    );
                });
                ui.add_space(4.0);
                if ui.button("➕  Add Source").clicked() {
                    self.show_add_source = true;
                    self.source_form = SourceForm::default();
                }
                ui.separator();

                let search_lower = self.sources_search.to_lowercase();
                let filtered: Vec<Source> = self
                    .sources
                    .iter()
                    .filter(|s| {
                        search_lower.is_empty()
                            || s.title.to_lowercase().contains(&search_lower)
                    })
                    .cloned()
                    .collect();

                ui.label(
                    egui::RichText::new(format!("{} / {} sources", filtered.len(), self.sources.len()))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(2.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for source in &filtered {
                            let year =
                                source.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                            let label = format!("{}{}", source.title, year);
                            ui.label(
                                egui::RichText::new(&label).color(egui::Color32::LIGHT_GRAY),
                            );
                            if let Some(a) = &source.author {
                                ui.label(
                                    egui::RichText::new(format!("   by {}", a))
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(100.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Select a source from the list, or add a new one.")
                        .color(egui::Color32::GRAY),
                );
            });
        });

        // ── Add Source window ──────────────────────────────────────────────────
        if self.show_add_source {
            let mut open = true;
            egui::Window::new("Add Source")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::Grid::new("add_source_grid")
                        .num_columns(2)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Title:");
                            ui.text_edit_singleline(&mut self.source_form.title);
                            ui.end_row();
                            ui.label("Author:");
                            ui.text_edit_singleline(&mut self.source_form.author);
                            ui.end_row();
                            ui.label("Publication:");
                            ui.text_edit_singleline(&mut self.source_form.publication);
                            ui.end_row();
                            ui.label("Year:");
                            ui.text_edit_singleline(&mut self.source_form.year_str);
                            ui.end_row();
                            ui.label("Repository:");
                            ui.text_edit_singleline(&mut self.source_form.repository);
                            ui.end_row();
                            ui.label("Notes:");
                            ui.text_edit_multiline(&mut self.source_form.notes);
                            ui.end_row();
                        });
                    ui.add_space(4.0);
                    if ui.button("Save").clicked() {
                        let year: Option<i32> =
                            self.source_form.year_str.trim().parse().ok();
                        let title = self.source_form.title.trim().to_string();
                        let author =
                            opt_str(&self.source_form.author).map(|s| s.to_string());
                        let publication =
                            opt_str(&self.source_form.publication).map(|s| s.to_string());
                        let repository =
                            opt_str(&self.source_form.repository).map(|s| s.to_string());
                        let notes =
                            opt_str(&self.source_form.notes).map(|s| s.to_string());
                        let mut saved = false;
                        if let Some(app) = &self.app {
                            match app.add_source(
                                &title,
                                author.as_deref(),
                                publication.as_deref(),
                                year,
                                repository.as_deref(),
                                notes.as_deref(),
                            ) {
                                Ok(_) => {
                                    self.status_msg = "Source added.".to_string();
                                    saved = true;
                                }
                                Err(e) => self.status_msg = e.to_string(),
                            }
                        }
                        if saved {
                            self.show_add_source = false;
                            self.refresh_sources();
                        }
                    }
                });
            if !open {
                self.show_add_source = false;
            }
        }
    }
}

// ── Reports tab ───────────────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_reports(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("reports_controls")
            .resizable(false)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Reports");
                ui.add_space(4.0);

                ui.label(egui::RichText::new("Person ID (for person reports):").small());
                ui.text_edit_singleline(&mut self.report_person_id_str);
                ui.label(egui::RichText::new("Generations:").small());
                ui.text_edit_singleline(&mut self.report_generations_str);
                ui.add_space(8.0);

                ui.label(egui::RichText::new("General").strong().small());
                if ui.button("  People List  ").clicked() {
                    self.run_report(|app| people_list_report(app.database()));
                }
                ui.add_space(4.0);

                ui.label(egui::RichText::new("Person-specific").strong().small());
                if ui.button("  Individual  ").clicked() {
                    self.run_person_report(|app, pid| individual_report(app.database(), pid));
                }
                if ui.button("  Ancestors (Ahnentafel)  ").clicked() {
                    let gens: u32 =
                        self.report_generations_str.trim().parse().unwrap_or(4);
                    self.run_person_report(move |app, pid| {
                        ancestor_report(app.database(), pid, gens)
                    });
                }
                if ui.button("  Descendants  ").clicked() {
                    let gens: u32 =
                        self.report_generations_str.trim().parse().unwrap_or(4);
                    self.run_person_report(move |app, pid| {
                        descendant_report(app.database(), pid, gens)
                    });
                }
                if ui.button("  Family Group Sheet  ").clicked() {
                    self.run_person_report(|app, pid| family_group_sheet(app.database(), pid));
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new("Search Notes").strong().small());
                ui.text_edit_singleline(&mut self.notes_search_query);
                if ui.button("  Search  ").clicked() {
                    self.run_notes_search();
                }

                if !self.notes_search_results.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} result(s)",
                            self.notes_search_results.len()
                        ))
                        .small()
                        .color(egui::Color32::GRAY),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("notes_scroll")
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for m in self.notes_search_results.clone().iter() {
                                let snippet = if m.notes.len() > 60 {
                                    format!("{}…", &m.notes[..59])
                                } else {
                                    m.notes.clone()
                                };
                                ui.label(
                                    egui::RichText::new(format!(
                                        "[{}] {}\n\"{}\"",
                                        m.kind, m.label, snippet
                                    ))
                                    .small(),
                                );
                                ui.separator();
                            }
                        });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Output");
                if !self.report_output.is_empty() && ui.button("📋  Copy").clicked() {
                    ui.output_mut(|o| o.copied_text = self.report_output.clone());
                    self.status_msg = "Report copied to clipboard.".to_string();
                }
            });
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.report_output)
                            .font(egui::FontId::monospace(13.0))
                            .desired_rows(40)
                            .desired_width(f32::INFINITY),
                    );
                });
        });
    }

    fn run_report(&mut self, f: impl Fn(&Application) -> kinforge_core::KinforgeResult<String>) {
        if let Some(app) = &self.app {
            self.report_output = f(app).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_person_report(
        &mut self,
        f: impl Fn(&Application, &PersonId) -> kinforge_core::KinforgeResult<String>,
    ) {
        let pid_str = self.report_person_id_str.trim().to_string();
        match PersonId::from_str(&pid_str) {
            Err(_) => {
                self.report_output =
                    "Error: enter a valid Person UUID in the Person ID field.".to_string();
            }
            Ok(pid) => {
                if let Some(app) = &self.app {
                    self.report_output = f(app, &pid).unwrap_or_else(|e| e.to_string());
                }
            }
        }
    }

    fn run_notes_search(&mut self) {
        let query = self.notes_search_query.clone();
        if let Some(app) = &self.app {
            self.notes_search_results = app.search_notes(&query).unwrap_or_default();
        }
    }
}

// ── Import / Export tab ───────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_import_export(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Import / Export");
            ui.add_space(12.0);

            // Import
            ui.group(|ui| {
                ui.label(egui::RichText::new("Import GEDCOM (.ged)").strong());
                ui.add_space(4.0);
                egui::Grid::new("import_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("File path:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.import_path_str)
                                .desired_width(400.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(4.0);
                if ui.button("  Import GEDCOM  ").clicked() {
                    self.do_import_gedcom();
                }
            });

            ui.add_space(12.0);

            // Export
            ui.group(|ui| {
                ui.label(egui::RichText::new("Export GEDCOM (.ged)").strong());
                ui.add_space(4.0);
                egui::Grid::new("export_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("File path:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.export_path_str)
                                .desired_width(400.0),
                        );
                        ui.end_row();
                    });
                ui.add_space(4.0);
                if ui.button("  Export GEDCOM  ").clicked() {
                    self.do_export_gedcom();
                }
            });

            if !self.ie_status.is_empty() {
                ui.add_space(12.0);
                let color = if self.ie_status.to_lowercase().contains("error") {
                    egui::Color32::RED
                } else {
                    egui::Color32::from_rgb(100, 220, 120)
                };
                ui.label(egui::RichText::new(&self.ie_status).color(color));
            }
        });
    }

    fn do_import_gedcom(&mut self) {
        let path = self.import_path_str.trim().to_string();
        match std::fs::read_to_string(&path) {
            Err(e) => self.ie_status = format!("Read error: {}", e),
            Ok(content) => {
                if let Some(app) = &self.app {
                    match import_gedcom(&content, app.database()) {
                        Ok(stats) => {
                            self.ie_status = format!(
                                "Imported: {} people, {} events, {} sources, {} relationships, {} skipped",
                                stats.people,
                                stats.events,
                                stats.sources,
                                stats.relationships,
                                stats.duplicates_skipped,
                            );
                            self.refresh_people();
                            self.refresh_sources();
                        }
                        Err(e) => self.ie_status = format!("Import error: {}", e),
                    }
                }
            }
        }
    }

    fn do_export_gedcom(&mut self) {
        let path = self.export_path_str.trim().to_string();
        if let Some(app) = &self.app {
            match std::fs::File::create(&path) {
                Err(e) => self.ie_status = format!("Create file error: {}", e),
                Ok(mut f) => match export_gedcom(app.database(), &mut f) {
                    Ok(()) => self.ie_status = format!("Exported GEDCOM to {}", path),
                    Err(e) => self.ie_status = format!("Export error: {}", e),
                },
            }
        }
    }
}

// ── Settings tab ──────────────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_settings(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Settings");
            ui.add_space(8.0);

            if let Some(app) = &self.app {
                egui::Grid::new("settings_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Database path:").strong());
                        ui.label(app.config.database_path.display().to_string());
                        ui.end_row();

                        ui.label(egui::RichText::new("Backup on open:").strong());
                        ui.label(app.config.backup_on_open.to_string());
                        ui.end_row();

                        ui.label(egui::RichText::new("Max backups:").strong());
                        ui.label(app.config.max_backups.to_string());
                        ui.end_row();
                    });

                let backup_dir = app
                    .config
                    .database_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("backups");

                ui.add_space(12.0);
                if ui
                    .button(format!("📁  Open backup folder: {}", backup_dir.display()))
                    .clicked()
                {
                    open_folder(&backup_dir);
                }
            }

            ui.add_space(16.0);
            ui.separator();
            ui.label(
                egui::RichText::new(
                    "To change settings, edit the config file and restart.\n\
                     Config location: ~/.config/kinforge/kinforge.toml (Linux) or equivalent.",
                )
                .small()
                .color(egui::Color32::GRAY),
            );

            ui.add_space(8.0);
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "Kinforge Family History v{}",
                    env!("CARGO_PKG_VERSION")
                ))
                .small()
                .color(egui::Color32::from_rgb(80, 140, 200)),
            );
        });
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn opt_str(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn parse_event_type(s: &str) -> EventType {
    match s.trim().to_lowercase().as_str() {
        "birth" => EventType::Birth,
        "death" => EventType::Death,
        "marriage" => EventType::Marriage,
        "divorce" => EventType::Divorce,
        "baptism" => EventType::Baptism,
        "burial" => EventType::Burial,
        "census" => EventType::Census,
        "residence" => EventType::Residence,
        other => EventType::Other(other.to_string()),
    }
}

fn parse_rel_type(s: &str) -> RelationshipType {
    match s.trim().to_lowercase().as_str() {
        "parentchild" | "parent_child" | "parent-child" => RelationshipType::ParentChild,
        "spouse" => RelationshipType::Spouse,
        _ => RelationshipType::Sibling,
    }
}

fn parse_date(s: &str) -> Option<EventDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // YYYY-MM-DD exact date
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return EventDate::from_parts("exact", Some(s), None);
    }
    // YYYY — store as Jan 1 of that year
    if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
        let date_str = format!("{}-01-01", s);
        return EventDate::from_parts("exact", Some(&date_str), None);
    }
    None
}

fn sex_label(sex: &Sex) -> &'static str {
    match sex {
        Sex::Male => "♂ Male",
        Sex::Female => "♀ Female",
        Sex::Unknown => "Unknown",
    }
}

fn sex_color(sex: &Sex) -> egui::Color32 {
    match sex {
        Sex::Male => egui::Color32::from_rgb(100, 160, 220),
        Sex::Female => egui::Color32::from_rgb(220, 130, 160),
        Sex::Unknown => egui::Color32::GRAY,
    }
}

fn event_type_color(event_type: &EventType) -> egui::Color32 {
    match event_type {
        EventType::Birth => egui::Color32::from_rgb(100, 200, 120),
        EventType::Death => egui::Color32::from_rgb(180, 100, 100),
        EventType::Marriage => egui::Color32::from_rgb(200, 170, 80),
        EventType::Divorce => egui::Color32::from_rgb(200, 120, 80),
        EventType::Baptism | EventType::Burial => egui::Color32::from_rgb(150, 150, 220),
        _ => egui::Color32::from_rgb(160, 160, 160),
    }
}

fn open_folder(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
