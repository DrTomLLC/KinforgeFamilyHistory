use anyhow::Result;
use clap::{Parser, Subcommand};
use kinforge_app::Application;
use kinforge_config::Config;
use std::path::PathBuf;

mod commands;
use commands::{
    citation::CitationCommands, config::ConfigCommands, event::EventCommands,
    export::ExportCommands, import::ImportCommands, media::MediaCommands,
    person::PersonCommands, place::PlaceCommands, relationship::RelationshipCommands,
    report::ReportCommands, search::SearchCommands, source::SourceCommands,
    task::TaskCommands,
};

#[derive(Parser)]
#[command(
    name = "kinforge",
    version,
    about = "Kinforge Family History — research-grade, local-first genealogy software",
    long_about = concat!(
        "Kinforge Family History\n\n",
        "A local-first genealogy program with no cloud dependency, no telemetry, and\n",
        "full source-based citation support. All data is stored in a local SQLite file.\n\n",
        "Quick start:\n",
        "  kinforge person add --given John --surname Smith --sex male\n",
        "  kinforge report stats\n",
    )
)]
struct Cli {
    /// Override the database file path
    #[arg(long, global = true, env = "KINFORGE_DB")]
    db: Option<PathBuf>,

    /// Override the config file path
    #[arg(long, global = true, env = "KINFORGE_CONFIG")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage people
    #[command(subcommand)]
    Person(PersonCommands),

    /// Manage events
    #[command(subcommand)]
    Event(EventCommands),

    /// Manage relationships between people
    #[command(subcommand)]
    Relationship(RelationshipCommands),

    /// Manage places
    #[command(subcommand)]
    Place(PlaceCommands),

    /// Manage sources (books, records, etc.)
    #[command(subcommand)]
    Source(SourceCommands),

    /// Manage citations (links between events and sources)
    #[command(subcommand)]
    Citation(CitationCommands),

    /// Generate reports
    #[command(subcommand)]
    Report(ReportCommands),

    /// Export data to other formats
    #[command(subcommand)]
    Export(ExportCommands),

    /// Import data from other formats
    #[command(subcommand)]
    Import(ImportCommands),

    /// Search the database
    #[command(subcommand)]
    Search(SearchCommands),

    /// Manage media attachments (photos, documents, audio, video)
    #[command(subcommand)]
    Media(MediaCommands),

    /// Manage configuration (show, init, set)
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Manage research tasks
    #[command(subcommand)]
    Task(TaskCommands),

    /// Show upcoming birthdays and anniversaries
    Reminders {
        /// Days ahead to look (default: 30)
        #[arg(long, default_value = "30")]
        days: u32,
    },

    /// Run data integrity checks and report issues
    Check,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Build config
    let mut config = if let Some(ref cfg_path) = cli.config {
        Config::load(cfg_path)?
    } else {
        Config::load_or_default()
    };

    // CLI --db flag overrides config
    if let Some(db_path) = cli.db {
        config.database_path = db_path;
    }

    // Handle the Config meta-command before opening DB (does not require DB access)
    if let Commands::Config(cmd) = cli.command {
        return commands::config::handle(cmd, &config);
    }

    let app = Application::open(config)?;

    match cli.command {
        Commands::Person(cmd) => commands::person::handle(cmd, &app)?,
        Commands::Event(cmd) => commands::event::handle(cmd, &app)?,
        Commands::Relationship(cmd) => commands::relationship::handle(cmd, &app)?,
        Commands::Place(cmd) => commands::place::handle(cmd, &app)?,
        Commands::Source(cmd) => commands::source::handle(cmd, &app)?,
        Commands::Citation(cmd) => commands::citation::handle(cmd, &app)?,
        Commands::Report(cmd) => commands::report::handle(cmd, &app)?,
        Commands::Export(cmd) => commands::export::handle(cmd, &app)?,
        Commands::Import(cmd) => commands::import::handle(cmd, &app)?,
        Commands::Search(cmd) => commands::search::handle(cmd, &app)?,
        Commands::Media(cmd) => commands::media::handle(cmd, &app)?,
        Commands::Task(cmd) => commands::task::handle(cmd, &app)?,
        Commands::Reminders { days } => commands::reminders::handle(days, &app)?,
        Commands::Check => commands::check::handle(&app)?,
        Commands::Config(_) => unreachable!(),
    }

    Ok(())
}
