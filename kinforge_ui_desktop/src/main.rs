#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use kinforge_app::Application;
use kinforge_config::Config;
use kinforge_core::models::*;
use kinforge_import_export::{export_gedcom, import_gedcom, DuplicateHandling, ImportOptions};
use kinforge_reports::{
    ahnentafel_table, descendant_report, family_group_sheet, individual_report, people_list_report,
};
use kinforge_storage::NoteMatch;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Kinforge Family History"),
        ..Default::default()
    };
    eframe::run_native(
        "Kinforge Family History",
        options,
        Box::new(|_cc| Box::new(KinforgeApp::new())),
    )
}

// ── Tabs ──────────────────────────────────────────────────────────────────────

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

// ── App ───────────────────────────────────────────────────────────────────────

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
    notes_search_results: Vec<NoteMatch>,

    // Import/Export
    import_path_str: String,
    export_path_str: String,
    on_duplicate: DuplicateHandling,
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
            on_duplicate: DuplicateHandling::Skip,
            ie_status: String::new(),
        }
    }

    fn open_db(&mut self) {
        let path = PathBuf::from(&self.db_path_str);
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
        // If no DB open yet, show opener
        if self.app.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(180.0);
                    ui.heading("Kinforge Family History");
                    ui.add_space(20.0);
                    ui.label("Database file:");
                    ui.add_sized(
                        [300.0, 24.0],
                        egui::TextEdit::singleline(&mut self.db_path_str),
                    );
                    ui.add_space(8.0);
                    if ui.button("Open / Create Database").clicked() {
                        self.open_db();
                    }
                    if let Some(err) = &self.open_error.clone() {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                });
            });
            return;
        }

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, Tab::People, "People");
                ui.selectable_value(&mut self.current_tab, Tab::Sources, "Sources");
                ui.selectable_value(&mut self.current_tab, Tab::Reports, "Reports");
                ui.selectable_value(&mut self.current_tab, Tab::ImportExport, "Import / Export");
                ui.selectable_value(&mut self.current_tab, Tab::Settings, "Settings");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status_msg.is_empty() {
                        ui.label(self.status_msg.as_str());
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
            .show(ctx, |ui| {
                ui.heading("People");
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.people_search);
                });
                if ui.button("+ Add Person").clicked() {
                    self.show_add_person = true;
                    self.person_form = PersonForm::default();
                }
                ui.separator();
                let search_lower = self.people_search.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let people: Vec<Person> = self.people.clone();
                    for person in &people {
                        let name = person.display_name();
                        if !search_lower.is_empty() && !name.to_lowercase().contains(&search_lower)
                        {
                            continue;
                        }
                        let selected = self.selected_person_id.as_ref() == Some(&person.id);
                        let label = format!("{} ({})", name, person.sex);
                        if ui.selectable_label(selected, &label).clicked() {
                            self.selected_person_id = Some(person.id.clone());
                            self.refresh_selected_person();
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let selected_id = self.selected_person_id.clone();
            if let Some(pid) = &selected_id {
                let person_opt = self.people.iter().find(|p| &p.id == pid).cloned();
                if let Some(person) = person_opt {
                    ui.heading(person.display_name());
                    ui.label(format!("ID: {}", person.id));
                    ui.label(format!("Sex: {}", person.sex));
                    if let Some(n) = &person.notes {
                        ui.label(format!("Notes: {}", n));
                    }

                    ui.separator();
                    ui.label("Names:");
                    for (i, name) in person.names.iter().enumerate() {
                        let g = name.given.as_deref().unwrap_or("");
                        let s = name.surname.as_deref().unwrap_or("");
                        ui.label(format!("  [{}] {} {} ({:?})", i, g, s, name.name_type));
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Events:");
                        if ui.button("+ Add Event").clicked() {
                            self.show_add_event = true;
                            self.event_form = EventForm::default();
                        }
                    });
                    for ev in self.person_events.clone().iter() {
                        let date_str = ev
                            .date
                            .as_ref()
                            .map(|d| format!("{}", d))
                            .unwrap_or_default();
                        ui.label(format!(
                            "  {:?}  {}  {}",
                            ev.event_type,
                            date_str,
                            ev.notes.as_deref().unwrap_or("")
                        ));
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Relationships:");
                        if ui.button("+ Add Relationship").clicked() {
                            self.show_add_relationship = true;
                        }
                    });
                    for rel in self.person_relationships.clone().iter() {
                        ui.label(format!(
                            "  {}: {} <-> {}",
                            rel.rel_type, rel.person1_id, rel.person2_id
                        ));
                    }

                    ui.separator();
                    if ui
                        .button(egui::RichText::new("Delete Person").color(egui::Color32::RED))
                        .clicked()
                    {
                        if let Some(app) = &self.app {
                            let _ = app.delete_person(pid);
                        }
                        self.selected_person_id = None;
                        self.person_events.clear();
                        self.person_relationships.clear();
                        self.refresh_people();
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a person from the list, or add a new one.");
                });
            }
        });

        // Add Person window
        if self.show_add_person {
            let mut open = true;
            egui::Window::new("Add Person")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Given name:");
                    ui.text_edit_singleline(&mut self.person_form.given);
                    ui.label("Surname:");
                    ui.text_edit_singleline(&mut self.person_form.surname);
                    ui.label("Sex (Male/Female/Unknown):");
                    ui.text_edit_singleline(&mut self.person_form.sex);
                    ui.label("Notes:");
                    ui.text_edit_multiline(&mut self.person_form.notes);
                    if ui.button("Save").clicked() {
                        let sex: Sex = self.person_form.sex.parse().unwrap_or(Sex::Unknown);
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

        // Add Event window
        if self.show_add_event {
            if let Some(pid) = self.selected_person_id.clone() {
                let mut open = true;
                egui::Window::new("Add Event")
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.label("Event type (Birth/Death/Marriage/Census/Residence):");
                        ui.text_edit_singleline(&mut self.event_form.event_type);
                        ui.label("Date (YYYY-MM-DD or YYYY):");
                        ui.text_edit_singleline(&mut self.event_form.date_str);
                        ui.label("Place:");
                        ui.text_edit_singleline(&mut self.event_form.place);
                        ui.label("Notes:");
                        ui.text_edit_multiline(&mut self.event_form.notes);
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

        // Add Relationship window
        if self.show_add_relationship {
            if let Some(pid) = self.selected_person_id.clone() {
                let mut open = true;
                egui::Window::new("Add Relationship")
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.label("Other person ID:");
                        ui.text_edit_singleline(&mut self.rel_person2_id_str);
                        ui.label("Type (ParentChild/Spouse/Sibling):");
                        ui.text_edit_singleline(&mut self.rel_type_str);
                        ui.label("Notes:");
                        ui.text_edit_multiline(&mut self.rel_notes);
                        if ui.button("Save").clicked() {
                            let rt = parse_rel_type(&self.rel_type_str);
                            let p2 = PersonId::from_str(self.rel_person2_id_str.trim())
                                .unwrap_or_else(|_| PersonId::new());
                            let notes = opt_str(&self.rel_notes);
                            let mut saved = false;
                            if let Some(app) = &self.app {
                                match app.add_relationship(rt, pid.clone(), p2, notes) {
                                    Ok(_) => {
                                        self.status_msg = "Relationship added.".to_string();
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
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Sources");
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.sources_search);
                });
                if ui.button("+ Add Source").clicked() {
                    self.show_add_source = true;
                    self.source_form = SourceForm::default();
                }
                ui.separator();
                let search_lower = self.sources_search.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for source in self.sources.clone().iter() {
                        if !search_lower.is_empty()
                            && !source.title.to_lowercase().contains(&search_lower)
                        {
                            continue;
                        }
                        let year = source.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                        ui.label(format!("{}{}", source.title, year));
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label("Select a source from the list, or add a new one.");
            });
        });

        if self.show_add_source {
            let mut open = true;
            egui::Window::new("Add Source")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Title:");
                    ui.text_edit_singleline(&mut self.source_form.title);
                    ui.label("Author:");
                    ui.text_edit_singleline(&mut self.source_form.author);
                    ui.label("Publication:");
                    ui.text_edit_singleline(&mut self.source_form.publication);
                    ui.label("Year:");
                    ui.text_edit_singleline(&mut self.source_form.year_str);
                    ui.label("Repository:");
                    ui.text_edit_singleline(&mut self.source_form.repository);
                    ui.label("Notes:");
                    ui.text_edit_multiline(&mut self.source_form.notes);
                    if ui.button("Save").clicked() {
                        let year: Option<i32> = self.source_form.year_str.trim().parse().ok();
                        let title = self.source_form.title.clone();
                        let author = opt_str(&self.source_form.author).map(|s| s.to_string());
                        let publication =
                            opt_str(&self.source_form.publication).map(|s| s.to_string());
                        let repository =
                            opt_str(&self.source_form.repository).map(|s| s.to_string());
                        let notes = opt_str(&self.source_form.notes).map(|s| s.to_string());
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
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Reports");
                ui.label("Person ID:");
                ui.text_edit_singleline(&mut self.report_person_id_str);
                ui.label("Generations:");
                ui.text_edit_singleline(&mut self.report_generations_str);
                ui.add_space(4.0);

                if ui.button("People List").clicked() {
                    self.run_people_list_report();
                }
                if ui.button("Individual Report").clicked() {
                    self.run_individual_report();
                }
                if ui.button("Ahnentafel Table").clicked() {
                    self.run_ahnentafel();
                }
                if ui.button("Family Group Sheet").clicked() {
                    self.run_family_group();
                }
                if ui.button("Descendants").clicked() {
                    self.run_descendants();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.label("Search notes:");
                ui.text_edit_singleline(&mut self.notes_search_query);
                if ui.button("Search").clicked() {
                    self.run_notes_search();
                }
                if !self.notes_search_results.is_empty() {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_source("notes_scroll")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for m in self.notes_search_results.clone().iter() {
                                let snippet = if m.notes.len() > 60 {
                                    format!("{}…", &m.notes[..59])
                                } else {
                                    m.notes.clone()
                                };
                                ui.label(format!(
                                    "[{}] {} — \"{}\"",
                                    m.entity_type, m.label, snippet
                                ));
                            }
                        });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Output");
                if !self.report_output.is_empty() && ui.button("Copy").clicked() {
                    ui.output_mut(|o| o.copied_text = self.report_output.clone());
                }
            });
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.report_output)
                        .font(egui::FontId::monospace(13.0))
                        .desired_rows(40)
                        .desired_width(f32::INFINITY),
                );
            });
        });
    }

    fn run_people_list_report(&mut self) {
        if let Some(app) = &self.app {
            self.report_output = people_list_report(&app.db).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_individual_report(&mut self) {
        let pid = PersonId::from_str(self.report_person_id_str.trim())
            .unwrap_or_else(|_| PersonId::new());
        if let Some(app) = &self.app {
            self.report_output = individual_report(&app.db, &pid).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_ahnentafel(&mut self) {
        let pid = PersonId::from_str(self.report_person_id_str.trim())
            .unwrap_or_else(|_| PersonId::new());
        let gens: u32 = self.report_generations_str.trim().parse().unwrap_or(4);
        if let Some(app) = &self.app {
            self.report_output =
                ahnentafel_table(&app.db, &pid, gens).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_family_group(&mut self) {
        let pid = PersonId::from_str(self.report_person_id_str.trim())
            .unwrap_or_else(|_| PersonId::new());
        if let Some(app) = &self.app {
            self.report_output =
                family_group_sheet(&app.db, &pid).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_descendants(&mut self) {
        let pid = PersonId::from_str(self.report_person_id_str.trim())
            .unwrap_or_else(|_| PersonId::new());
        let gens: u32 = self.report_generations_str.trim().parse().unwrap_or(4);
        if let Some(app) = &self.app {
            self.report_output =
                descendant_report(&app.db, &pid, gens).unwrap_or_else(|e| e.to_string());
        }
    }

    fn run_notes_search(&mut self) {
        if let Some(app) = &self.app {
            self.notes_search_results = app
                .search_notes(&self.notes_search_query.clone())
                .unwrap_or_default();
        }
    }
}

// ── Import / Export tab ───────────────────────────────────────────────────────

impl KinforgeApp {
    fn tab_import_export(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Import / Export");
            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Import GEDCOM (.ged)");
                ui.label("File path:");
                ui.text_edit_singleline(&mut self.import_path_str);
                ui.label("On duplicate:");
                egui::ComboBox::from_id_source("on_dup")
                    .selected_text(format!("{:?}", self.on_duplicate))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.on_duplicate,
                            DuplicateHandling::Skip,
                            "Skip",
                        );
                        ui.selectable_value(
                            &mut self.on_duplicate,
                            DuplicateHandling::Merge,
                            "Merge",
                        );
                        ui.selectable_value(
                            &mut self.on_duplicate,
                            DuplicateHandling::Add,
                            "Add (always insert)",
                        );
                    });
                if ui.button("Import GEDCOM").clicked() {
                    self.do_import_gedcom();
                }
            });

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label("Export GEDCOM (.ged)");
                ui.label("File path:");
                ui.text_edit_singleline(&mut self.export_path_str);
                if ui.button("Export GEDCOM").clicked() {
                    self.do_export_gedcom();
                }
            });

            if !self.ie_status.is_empty() {
                ui.add_space(8.0);
                ui.label(self.ie_status.as_str());
            }
        });
    }

    fn do_import_gedcom(&mut self) {
        let path = self.import_path_str.trim().to_string();
        match std::fs::read_to_string(&path) {
            Err(e) => self.ie_status = format!("Read error: {}", e),
            Ok(content) => {
                if let Some(app) = &self.app {
                    let opts = ImportOptions {
                        on_duplicate: self.on_duplicate,
                    };
                    match import_gedcom(&content, &app.db, &opts) {
                        Ok(stats) => {
                            self.ie_status = format!(
                                "Imported: {} people, {} events, {} sources, {} skipped, {} merged",
                                stats.people,
                                stats.events,
                                stats.sources,
                                stats.skipped_duplicates,
                                stats.merged_people
                            );
                        }
                        Err(e) => self.ie_status = format!("Import error: {}", e),
                    }
                }
                self.refresh_people();
                self.refresh_sources();
            }
        }
    }

    fn do_export_gedcom(&mut self) {
        let path = self.export_path_str.trim().to_string();
        if let Some(app) = &self.app {
            match std::fs::File::create(&path) {
                Err(e) => self.ie_status = format!("Create error: {}", e),
                Ok(mut f) => match export_gedcom(&app.db, &mut f) {
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
            if let Some(app) = &self.app {
                ui.label(format!("Database: {}", app.config.database_path.display()));
                ui.label(format!("Backup on open: {}", app.config.backup_on_open));
                ui.label(format!("Max backups: {}", app.config.max_backups));
                let backup_dir = app
                    .config
                    .database_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .join("backups");
                if ui
                    .button(format!("Open backup folder: {}", backup_dir.display()))
                    .clicked()
                {
                    open_folder(&backup_dir);
                }
            }
            ui.add_space(16.0);
            ui.separator();
            ui.label("To change settings, edit the config file and restart.");
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
    // YYYY-MM-DD
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return EventDate::from_parts("", Some(s), None);
    }
    // YYYY — convert to Jan 1 of that year for storage
    if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
        let date_str = format!("{}-01-01", s);
        return EventDate::from_parts("", Some(&date_str), None);
    }
    None
}

fn open_folder(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(path).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}
