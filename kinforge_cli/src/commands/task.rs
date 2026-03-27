use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{TaskPriority, TaskStatus};

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Add a new research task
    Add {
        /// Task description
        description: String,
        /// Priority: low, medium, high (default: medium)
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Link to a person (ID or prefix)
        #[arg(long)]
        person: Option<String>,
        /// Optional notes
        #[arg(long)]
        notes: Option<String>,
    },
    /// Show a specific task
    Show {
        /// Task ID or prefix
        id: String,
    },
    /// List all tasks (pending and in-progress first)
    List {
        /// Filter by status: pending, in-progress, done
        #[arg(long)]
        status: Option<String>,
        /// Filter by priority: low, medium, high
        #[arg(long)]
        priority: Option<String>,
        /// Filter to tasks for a specific person (ID or prefix)
        #[arg(long)]
        person: Option<String>,
    },
    /// Update a task
    Update {
        /// Task ID or prefix
        id: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        person: Option<String>,
        /// Remove the person link
        #[arg(long)]
        clear_person: bool,
    },
    /// Mark a task as done
    Done {
        /// Task ID or prefix
        id: String,
    },
    /// Start working on a task (sets status to in-progress)
    Start {
        /// Task ID or prefix
        id: String,
    },
    /// Delete a task permanently
    Delete {
        /// Task ID or prefix
        id: String,
    },
}

pub fn handle(cmd: TaskCommands, app: &Application) -> Result<()> {
    match cmd {
        TaskCommands::Add { description, priority, person, notes } => {
            let prio: TaskPriority = priority.parse()?;
            let person_id = person
                .as_deref()
                .map(|p| app.resolve_person_id(p))
                .transpose()?;
            let task = app.add_task(&description, person_id, prio, notes.as_deref())?;
            println!(
                "{} {} {}",
                "Added task".green(),
                task.id.as_str()[..8].bright_black(),
                task.description.bold()
            );
        }

        TaskCommands::Show { id } => {
            let tid = app.resolve_task_id(&id)?;
            let t = app.get_task(&tid)?;
            print_task(&t, app);
        }

        TaskCommands::List { status, priority, person } => {
            let mut tasks = if let Some(ref p) = person {
                let pid = app.resolve_person_id(p)?;
                app.list_tasks_for_person(&pid)?
            } else {
                app.list_tasks()?
            };

            // Filter by status
            if let Some(ref s) = status {
                let filter: TaskStatus = s.parse()?;
                tasks.retain(|t| t.status == filter);
            }
            // Filter by priority
            if let Some(ref p) = priority {
                let filter: TaskPriority = p.parse()?;
                tasks.retain(|t| t.priority == filter);
            }

            if tasks.is_empty() {
                println!("{}", "No tasks found.".bright_black());
                return Ok(());
            }

            let pending: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::Pending).collect();
            let in_progress: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).collect();
            let done: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::Done).collect();

            println!(
                "{}\n",
                "  Research Tasks  ".bold().bright_cyan().on_black()
            );

            if !in_progress.is_empty() {
                println!("{}", "In Progress:".yellow().bold());
                for t in &in_progress { print_task_row(t, app); }
                println!();
            }
            if !pending.is_empty() {
                println!("{}", "Pending:".cyan().bold());
                for t in &pending { print_task_row(t, app); }
                println!();
            }
            if !done.is_empty() {
                println!("{}", "Done:".bright_black().bold());
                for t in &done { print_task_row(t, app); }
            }
        }

        TaskCommands::Update { id, description, priority, status, notes, person, clear_person } => {
            let tid = app.resolve_task_id(&id)?;
            let mut t = app.get_task(&tid)?;
            if let Some(d) = description { t.description = d; }
            if let Some(p) = priority { t.priority = p.parse()?; }
            if let Some(s) = status { t.status = s.parse()?; }
            if let Some(n) = notes { t.notes = Some(n); }
            if clear_person {
                t.person_id = None;
            } else if let Some(p) = person {
                t.person_id = Some(app.resolve_person_id(&p)?);
            }
            t.touch();
            let updated = app.update_task(t)?;
            println!(
                "{} {}",
                "Updated task".green(),
                updated.description.bold()
            );
        }

        TaskCommands::Done { id } => {
            let tid = app.resolve_task_id(&id)?;
            let t = app.complete_task(&tid)?;
            println!(
                "{} {} {}",
                "\u{2713}".bright_green().bold(),
                "Done:".green(),
                t.description.bold()
            );
        }

        TaskCommands::Start { id } => {
            let tid = app.resolve_task_id(&id)?;
            let mut t = app.get_task(&tid)?;
            t.status = TaskStatus::InProgress;
            t.touch();
            let updated = app.update_task(t)?;
            println!(
                "{} {}",
                "Started:".yellow(),
                updated.description.bold()
            );
        }

        TaskCommands::Delete { id } => {
            let tid = app.resolve_task_id(&id)?;
            let t = app.get_task(&tid)?;
            app.delete_task(&tid)?;
            println!("{} {}", "Deleted task".red(), t.description.bold());
        }
    }
    Ok(())
}

fn status_badge(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "[ ]".bright_black().to_string(),
        TaskStatus::InProgress => "[~]".yellow().to_string(),
        TaskStatus::Done => "[✓]".bright_green().to_string(),
    }
}

fn priority_badge(priority: &TaskPriority) -> String {
    match priority {
        TaskPriority::High => "HIGH  ".red().bold().to_string(),
        TaskPriority::Medium => "MED   ".yellow().to_string(),
        TaskPriority::Low => "LOW   ".bright_black().to_string(),
    }
}

fn print_task_row(t: &kinforge_core::models::Task, app: &Application) {
    let person_str = t
        .person_id
        .as_ref()
        .and_then(|pid| app.get_person(pid).ok())
        .map(|p| format!(" ({})", p.display_name().bright_black()))
        .unwrap_or_default();
    println!(
        "  {} {} {} {}{}",
        status_badge(&t.status),
        priority_badge(&t.priority),
        t.id.as_str()[..8].bright_black(),
        t.description.bold(),
        person_str
    );
}

fn print_task(t: &kinforge_core::models::Task, app: &Application) {
    println!(
        "\n  {} {} {}",
        status_badge(&t.status),
        t.description.bold(),
        format!("[{}]", t.priority).yellow()
    );
    println!("  {} {}", "ID:".cyan(), t.id.as_str().bright_black());
    println!("  {} {}", "Status:".cyan(), t.status.to_string().yellow());
    println!("  {} {}", "Priority:".cyan(), t.priority.to_string());
    if let Some(ref pid) = t.person_id {
        let name = app
            .get_person(pid)
            .map(|p| p.display_name())
            .unwrap_or_else(|_| pid.to_string());
        println!("  {} {}", "Person:".cyan(), name.bold());
    }
    if let Some(ref notes) = t.notes {
        println!("  {} {}", "Notes:".cyan(), notes);
    }
    println!("  {} {}", "Created:".cyan(), t.created.bright_black());
    println!("  {} {}", "Updated:".cyan(), t.updated.bright_black());
}
