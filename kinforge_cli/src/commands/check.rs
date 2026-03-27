use anyhow::Result;
use colored::Colorize;
use kinforge_app::Application;

pub fn handle(app: &Application) -> Result<()> {
    let issues = app.check_integrity()?;

    if issues.is_empty() {
        println!("{}", "  No integrity issues found.  ".bold().bright_green().on_black());
        return Ok(());
    }

    println!(
        "{}\n",
        format!("  {} issue(s) found  ", issues.len())
            .bold()
            .bright_yellow()
            .on_black()
    );

    for issue in &issues {
        let severity_label = match issue.severity {
            "error" => "ERROR  ".bright_red().bold().to_string(),
            "warning" => "WARNING".yellow().bold().to_string(),
            _ => issue.severity.to_string(),
        };
        println!(
            "  {} {} {} {}",
            severity_label,
            format!("[{}]", issue.entity_type).bright_black(),
            issue.id.bright_black(),
            issue.message
        );
    }

    println!(
        "\n{}",
        format!("  Run 'kinforge report stats' to see database totals.").bright_black()
    );

    Ok(())
}
