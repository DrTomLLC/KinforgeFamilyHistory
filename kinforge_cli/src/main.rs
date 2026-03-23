use anyhow::Result;
use clap::{Parser, Subcommand};
use kinforge_app::Application;
use kinforge_config::Config;

mod commands;
use commands::{
    citation::CitationCommands,
    event::EventCommands,
    export::ExportCommands,
    import::ImportCommands,
    person::PersonCommands,
    place::PlaceCommands,
    relationship::RelationshipCommands,
    report::ReportCommands,
    search::SearchCommands,
    source::SourceCommands,
};

#[derive(Parser)]
#[command(
    name = "kinforge",
    version,
    about = "Kinforge Family History — research-grade, local-first genealogy software",
    long_about = None
)]
struct Cli {
    /// Path to the database file (overrides config)
    #[arg(long, global = true)]
    db: Option<String>,

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

    /// Manage relationships
    #[command(subcommand)]
    Relationship(RelationshipCommands),

    /// Manage places
    #[command(subcommand)]
    Place(PlaceCommands),

    /// Manage sources
    #[command(subcommand)]
    Source(SourceCommands),

    /// Manage citations
    #[command(subcommand)]
    Citation(CitationCommands),

    /// Generate reports
    #[command(subcommand)]
    Report(ReportCommands),

    /// Export data
    #[command(subcommand)]
    Export(ExportCommands),

    /// Import data
    #[command(subcommand)]
    Import(ImportCommands),

    /// Search the database
    #[command(subcommand)]
    Search(SearchCommands),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut config = Config::load_or_default();
    if let Some(db_path) = cli.db {
        config.database_path = db_path.into();
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
    }

    Ok(())
}
