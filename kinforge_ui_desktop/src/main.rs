use eframe::egui;
use kinforge_app::Application;
use kinforge_core::models::{Event, EventType, Person, Sex};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Kinforge Family History")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    let config = kinforge_config::Config::load_or_default();
    let app = match Application::open(config) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to open database: {e}");
            std::process::exit(1);
        }
    };

    eframe::run_native(
        "Kinforge Family History",
        options,
        Box::new(|cc| Ok(Box::new(KinforgeApp::new(cc, app)))),
    )
}

// ── Application state ─────────────────────────────────────────────────────────

struct KinforgeApp {
    app: Application,

    // People panel
    people: Vec<Person>,
    people_search: String,
    selected_person_idx: Option<usize>,

    // Detail data for selected person
    detail_events: Vec<Event>,
    detail_rels: Vec<(String, String)>, // (label, other name)

    // Stats
    people_count: usize,
    event_count: usize,
    source_count: usize,
    task_count: usize,

    // Status bar message
    status: String,
}

impl KinforgeApp {
    fn new(_cc: &eframe::CreationContext<'_>, app: Application) -> Self {
        let people = app.list_people().unwrap_or_default();
        let stats = app.stats().ok();
        let people_count = stats.as_ref().map(|s| s.people as usize).unwrap_or(people.len());
        let event_count = stats.as_ref().map(|s| s.events as usize).unwrap_or(0);
        let source_count = stats.as_ref().map(|s| s.sources as usize).unwrap_or(0);
        let task_count = app.list_tasks().map(|t| t.len()).unwrap_or(0);

        Self {
            app,
            people,
            people_search: String::new(),
            selected_person_idx: None,
            detail_events: vec![],
            detail_rels: vec![],
            people_count,
            event_count,
            source_count,
            task_count,
            status: "Ready".to_string(),
        }
    }

    fn reload_people(&mut self) {
        self.people = self.app.list_people().unwrap_or_default();
        if let Ok(s) = self.app.stats() {
            self.people_count = s.people as usize;
            self.event_count = s.events as usize;
            self.source_count = s.sources as usize;
        }
        self.task_count = self.app.list_tasks().map(|t| t.len()).unwrap_or(0);
        self.selected_person_idx = None;
        self.detail_events.clear();
        self.detail_rels.clear();
    }

    fn select_person(&mut self, idx: usize) {
        self.selected_person_idx = Some(idx);
        if let Some(p) = self.filtered_people().get(idx).cloned() {
            self.detail_events = self.app.list_events_for_person(&p.id).unwrap_or_default();
            let rels = self.app.list_relationships_for_person(&p.id).unwrap_or_default();
            self.detail_rels = rels.into_iter().filter_map(|rel| {
                let other_id = if rel.person1_id == p.id { &rel.person2_id } else { &rel.person1_id };
                let other_name = self.app.get_person(other_id).map(|o| o.display_name()).ok()?;
                let label = rel.rel_type.to_string();
                Some((label, other_name))
            }).collect();
        }
    }

    fn filtered_people(&self) -> Vec<Person> {
        let q = self.people_search.to_lowercase();
        if q.is_empty() {
            self.people.clone()
        } else {
            self.people.iter()
                .filter(|p| p.display_name().to_lowercase().contains(&q))
                .cloned()
                .collect()
        }
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
}

impl eframe::App for KinforgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Top menu bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Refresh").clicked() {
                        self.reload_people();
                        self.status = "Data refreshed.".to_string();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.status = format!(
                            "Kinforge Family History v{} — local-first genealogy software",
                            env!("CARGO_PKG_VERSION")
                        );
                        ui.close_menu();
                    }
                });

                // Stats summary in menu bar
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            " {} people · {} events · {} sources · {} tasks ",
                            self.people_count, self.event_count,
                            self.source_count, self.task_count
                        ))
                        .color(egui::Color32::from_rgb(150, 150, 150))
                        .small(),
                    );
                });
            });
        });

        // ── Bottom status bar ─────────────────────────────────────────────────
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(&self.status)
                        .color(egui::Color32::from_rgb(140, 140, 140))
                        .small(),
                );
            });
        });

        // ── Left panel: People list ───────────────────────────────────────────
        egui::SidePanel::left("people_panel")
            .resizable(true)
            .default_width(260.0)
            .min_width(180.0)
            .show(ctx, |ui| {
                ui.heading("People");
                ui.separator();

                // Search box
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.people_search)
                            .hint_text("Search…")
                            .desired_width(f32::INFINITY),
                    );
                    if resp.changed() {
                        self.selected_person_idx = None;
                        self.detail_events.clear();
                        self.detail_rels.clear();
                    }
                });

                ui.separator();

                let filtered = self.filtered_people();
                let count = filtered.len();
                ui.label(
                    egui::RichText::new(format!("{} / {} people", count, self.people.len()))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(2.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, person) in filtered.iter().enumerate() {
                            let is_selected = self.selected_person_idx == Some(i);
                            let name = person.display_name();
                            let sex_col = Self::sex_color(&person.sex);

                            let resp = ui.selectable_label(
                                is_selected,
                                egui::RichText::new(&name).color(
                                    if is_selected { egui::Color32::WHITE } else { egui::Color32::LIGHT_GRAY }
                                ),
                            );

                            if resp.clicked() {
                                self.select_person(i);
                                self.status = format!("Selected: {}", name);
                            }

                            // Sex badge overlay — drawn as a small coloured dot
                            let rect = resp.rect;
                            let dot_pos = egui::pos2(
                                rect.right() - 10.0,
                                rect.center().y,
                            );
                            ui.painter().circle_filled(dot_pos, 4.0, sex_col);
                        }
                    });
            });

        // ── Right panel: Stats sidebar ────────────────────────────────────────
        egui::SidePanel::right("stats_panel")
            .resizable(true)
            .default_width(200.0)
            .min_width(150.0)
            .show(ctx, |ui| {
                ui.heading("Database");
                ui.separator();

                let rows: &[(&str, usize)] = &[
                    ("People", self.people_count),
                    ("Events", self.event_count),
                    ("Sources", self.source_count),
                    ("Tasks", self.task_count),
                ];

                for (label, value) in rows {
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(value.to_string())
                                    .color(egui::Color32::from_rgb(100, 200, 120))
                                    .strong(),
                            );
                        });
                    });
                }

                ui.add_space(12.0);
                ui.separator();
                ui.label(egui::RichText::new("Quick Actions").small().strong());
                ui.add_space(4.0);

                if ui.button("🔄  Refresh Data").clicked() {
                    self.reload_people();
                    self.status = "Data refreshed.".to_string();
                }

                ui.add_space(4.0);
                if ui.button("📊  Open TUI (shell)").clicked() {
                    self.status = "Run `kinforge tui` in a terminal for the full TUI.".to_string();
                }

                ui.add_space(12.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("Legend")
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(2.0);
                for (col, label) in [
                    (egui::Color32::from_rgb(100, 160, 220), "Male"),
                    (egui::Color32::from_rgb(220, 130, 160), "Female"),
                    (egui::Color32::GRAY, "Unknown sex"),
                ] {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(10.0, 10.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().circle_filled(rect.center(), 4.0, col);
                        ui.label(egui::RichText::new(label).small());
                    });
                }
            });

        // ── Central panel: Person detail ──────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(idx) = self.selected_person_idx {
                let filtered = self.filtered_people();
                if let Some(person) = filtered.get(idx) {
                    let name = person.display_name();
                    ui.horizontal(|ui| {
                        ui.heading(&name);
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(Self::sex_label(&person.sex))
                                .color(Self::sex_color(&person.sex))
                                .strong(),
                        );
                    });
                    ui.separator();

                    if let Some(ref notes) = person.notes {
                        if !notes.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("📝 {}", notes))
                                    .italics()
                                    .color(egui::Color32::from_rgb(180, 180, 180)),
                            );
                            ui.add_space(4.0);
                        }
                    }

                    // ── Events ────────────────────────────────────────────────
                    ui.collapsing(
                        egui::RichText::new(format!("Events ({})", self.detail_events.len()))
                            .strong(),
                        |ui| {
                            if self.detail_events.is_empty() {
                                ui.label(
                                    egui::RichText::new("No events recorded.")
                                        .color(egui::Color32::GRAY)
                                        .italics(),
                                );
                            }
                            egui::Grid::new("events_grid")
                                .num_columns(3)
                                .striped(true)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("Type").strong());
                                    ui.label(egui::RichText::new("Date").strong());
                                    ui.label(egui::RichText::new("Notes").strong());
                                    ui.end_row();
                                    for event in &self.detail_events {
                                        let type_col = event_type_color(&event.event_type);
                                        ui.label(
                                            egui::RichText::new(event.event_type.to_string())
                                                .color(type_col),
                                        );
                                        let date_str = event.date.as_ref()
                                            .map(|d| d.to_string())
                                            .unwrap_or_else(|| "—".to_string());
                                        ui.label(&date_str);
                                        let note = event.notes.as_deref().unwrap_or("");
                                        ui.label(
                                            egui::RichText::new(note)
                                                .color(egui::Color32::GRAY)
                                                .small(),
                                        );
                                        ui.end_row();
                                    }
                                });
                        },
                    );

                    ui.add_space(8.0);

                    // ── Relationships ─────────────────────────────────────────
                    ui.collapsing(
                        egui::RichText::new(format!("Relationships ({})", self.detail_rels.len()))
                            .strong(),
                        |ui| {
                            if self.detail_rels.is_empty() {
                                ui.label(
                                    egui::RichText::new("No relationships recorded.")
                                        .color(egui::Color32::GRAY)
                                        .italics(),
                                );
                            }
                            egui::Grid::new("rels_grid")
                                .num_columns(2)
                                .striped(true)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    for (label, other_name) in &self.detail_rels {
                                        ui.label(
                                            egui::RichText::new(label)
                                                .color(egui::Color32::from_rgb(180, 140, 220)),
                                        );
                                        ui.label(other_name);
                                        ui.end_row();
                                    }
                                });
                        },
                    );
                } else {
                    // Selection out of range after filter changed
                    self.selected_person_idx = None;
                }
            } else {
                // No person selected
                ui.add_space(80.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Kinforge Family History")
                            .size(28.0)
                            .color(egui::Color32::from_rgb(80, 140, 200)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Select a person from the list to view details.")
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(24.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{} people · {} events · {} sources",
                            self.people_count, self.event_count, self.source_count
                        ))
                        .color(egui::Color32::from_rgb(100, 180, 120))
                        .size(18.0),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new(
                            "Use `kinforge tui` in a terminal for the full interactive interface\n\
                             or `kinforge --help` for the complete CLI.",
                        )
                        .small()
                        .color(egui::Color32::from_rgb(110, 110, 110)),
                    );
                });
            }
        });
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
