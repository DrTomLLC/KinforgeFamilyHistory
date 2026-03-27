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
use kinforge_core::models::RelationshipType;
use ratatui::{backend::CrosstermBackend, Terminal};
use state::InputMode;
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
                    events::Action::OpenDetail(pid) => {
                        state.detail_events =
                            app.list_events_for_person(&pid).unwrap_or_default();
                        state.detail_rel_rows =
                            build_rel_rows(app, &pid, &state.people);
                        state.detail_person_id = Some(pid);
                        state.detail_open = true;
                        state.detail_scroll = 0;
                        state.mode = InputMode::Detail;
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

fn build_rel_rows(
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

            // Look up name from cached list; fall back to UUID prefix
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
