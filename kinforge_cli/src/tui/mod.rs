mod events;
mod state;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kinforge_app::Application;
use kinforge_core::models::{RelationshipType, TaskPriority};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

pub fn handle(app: &Application) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, app);

    // Always restore terminal even on error
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &Application,
) -> Result<()> {
    let mut state = state::TuiState::new(app)?;

    loop {
        terminal.draw(|frame| ui::draw(frame, &state))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match events::handle_key(&mut state, key) {
                    events::Action::Quit => break,

                    events::Action::OpenPersonDetail(pid) => {
                        state.detail_events =
                            app.list_events_for_person(&pid).unwrap_or_default();
                        state.detail_rel_rows =
                            build_person_rel_rows(app, &pid, &state.people);
                        state.detail_person_id = Some(pid);
                        state.detail_open = true;
                        state.detail_scroll = 0;
                    }

                    events::Action::OpenSourceDetail(sid) => {
                        state.source_detail_citations =
                            build_source_citation_rows(app, &sid);
                        state.source_detail_open = true;
                        state.source_detail_scroll = 0;
                    }

                    events::Action::CompleteTask(tid) => {
                        let _ = app.complete_task(&tid);
                        state.reload_tasks(app);
                    }

                    events::Action::CreateTask(desc) => {
                        let _ = app.add_task(&desc, None, TaskPriority::Medium, None);
                        state.reload_tasks(app);
                    }

                    events::Action::CycleTaskPriority(tid) => {
                        if let Ok(mut task) = app.get_task(&tid) {
                            task.priority = match task.priority {
                                TaskPriority::Low => TaskPriority::Medium,
                                TaskPriority::Medium => TaskPriority::High,
                                TaskPriority::High => TaskPriority::Low,
                            };
                            task.touch();
                            let _ = app.update_task(task);
                            state.reload_tasks(app);
                        }
                    }

                    events::Action::DeleteTask(tid) => {
                        let _ = app.delete_task(&tid);
                        state.reload_tasks(app);
                    }

                    events::Action::CreatePerson(given, surname) => {
                        let g = if given.is_empty() { None } else { Some(given.as_str()) };
                        let s = if surname.is_empty() { None } else { Some(surname.as_str()) };
                        let _ = app.add_person(g, s, kinforge_core::models::Sex::Unknown, None);
                        state.reload_people(app);
                    }

                    events::Action::CreateSource(title, author) => {
                        let a = if author.is_empty() { None } else { Some(author.as_str()) };
                        let _ = app.add_source(&title, a, None, None, None, None);
                        state.reload_sources(app);
                    }

                    events::Action::DeletePerson(pid) => {
                        let _ = app.delete_person(&pid);
                        // Close detail if it was showing the deleted person
                        if state.detail_person_id.as_ref() == Some(&pid) {
                            state.detail_open = false;
                            state.detail_person_id = None;
                        }
                        state.reload_people(app);
                    }

                    events::Action::None => {}
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

    Ok(())
}

fn build_person_rel_rows(
    app: &Application,
    focused_pid: &kinforge_core::models::PersonId,
    people_cache: &[state::PersonRow],
) -> Vec<(String, String)> {
    let rels = app
        .list_relationships_for_person(focused_pid)
        .unwrap_or_default();

    rels.into_iter()
        .map(|rel| {
            let is_person1 = &rel.person1_id == focused_pid;
            let other_id = if is_person1 {
                &rel.person2_id
            } else {
                &rel.person1_id
            };

            let other_name = people_cache
                .iter()
                .find(|p| &p.id == other_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| other_id.to_string());

            let label = match (&rel.rel_type, is_person1) {
                (RelationshipType::ParentChild, true) => "Parent of",
                (RelationshipType::ParentChild, false) => "Child of",
                (RelationshipType::Spouse, _) => "Spouse of",
                (RelationshipType::Sibling, _) => "Sibling of",
                (RelationshipType::AdoptiveParent, true) => "Adoptive parent of",
                (RelationshipType::AdoptiveParent, false) => "Adopted by",
                (RelationshipType::Godparent, true) => "Godparent of",
                (RelationshipType::Godparent, false) => "Godchild of",
                (RelationshipType::HalfSibling, _) => "Half-sibling of",
                (RelationshipType::StepParent, true) => "Stepparent of",
                (RelationshipType::StepParent, false) => "Stepchild of",
                (RelationshipType::Foster, true) => "Foster parent of",
                (RelationshipType::Foster, false) => "Foster child of",
            };

            (label.to_string(), other_name)
        })
        .collect()
}

fn build_source_citation_rows(
    app: &Application,
    source_id: &kinforge_core::models::SourceId,
) -> Vec<(String, String)> {
    let citations = app
        .list_citations_for_source(source_id)
        .unwrap_or_default();

    citations
        .into_iter()
        .map(|cit| {
            // Build a human label for the cited event
            let event_label = app
                .get_event(&cit.event_id)
                .map(|e| {
                    let person_name = app
                        .get_person(&e.person_id)
                        .map(|p| p.display_name())
                        .unwrap_or_else(|_| "?".to_string());
                    format!("{} — {}", person_name, e.event_type)
                })
                .unwrap_or_else(|_| cit.event_id.to_string());

            let page_str = cit.page.unwrap_or_default();
            (event_label, page_str)
        })
        .collect()
}
