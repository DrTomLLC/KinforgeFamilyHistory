use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List all available built-in plugins
    List,
}

pub fn handle(cmd: PluginCommands) -> Result<()> {
    match cmd {
        PluginCommands::List => {
            println!(
                "\n{}\n{}",
                "  Built-in Plugins  ".bold().bright_cyan().on_black(),
                "─".repeat(48).bright_black()
            );

            let builtins: &[(&str, &str, &str)] = &[
                (
                    "Console Logger",
                    "builtin.console_log",
                    "Logs every database event (person added, event added, …) to stderr.",
                ),
                (
                    "Event Counter",
                    "builtin.event_counter",
                    "Counts adds per entity type; prints a summary line on unload.",
                ),
            ];

            for (name, id, desc) in builtins {
                println!(
                    "\n  {}  v{}\n  {}\n  {}",
                    name.bold(),
                    VERSION.bright_black(),
                    id.bright_black(),
                    desc
                );
            }
            println!();
            println!(
                "  {}",
                "To use a plugin: call app.register_plugin(Box::new(plugin)) in your application."
                    .bright_black()
            );
            println!();
        }
    }
    Ok(())
}
