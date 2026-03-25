use anyhow::Result;
use clap::{Parser, Subcommand};
use kinforge_app::Application;
use kinforge_config::Config;
use std::path::PathBuf;

mod commands;
use commands::{
    citation::CitationCommands, event::EventCommands, export::ExportCommands,
    import::ImportCommands, person::PersonCommands, place::PlaceCommands,
    relationship::RelationshipCommands, report::ReportCommands, search::SearchCommands,
    source::SourceCommands,
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

    /// Print the active configuration and data paths
    Config,
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

    // Handle the Config meta-command before opening DB
    if matches!(cli.command, Commands::Config) {
        println!("Database path: {}", config.database_path.display());
        println!("Backup on open: {}", config.backup_on_open);
        println!("Max backups:    {}", config.max_backups);
        println!("Log level:      {}", config.log_level);
        if let Some(ref dir) = config.default_export_dir {
            println!("Export dir:     {}", dir.display());
        }
        if let Some(p) = Config::default_config_path() {
            println!("Config file:    {}", p.display());
        }
        return Ok(());
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
        Commands::Config => unreachable!(),
    }

    Ok(())
}
