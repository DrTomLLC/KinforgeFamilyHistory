use anyhow::Result;
use chrono::{Datelike, Duration, Local, NaiveDate};
use colored::Colorize;
use kinforge_app::Application;
use kinforge_core::models::{EventDate, EventType};

pub fn handle(days: u32, app: &Application) -> Result<()> {
    let today = Local::now().date_naive();
    let window_end = today + Duration::days(days as i64);

    let mut reminders: Vec<(i64, String, String, NaiveDate)> = Vec::new();

    let people = app.list_people()?;
    for person in &people {
        let events = app.list_events_for_person(&person.id)?;
        for event in &events {
            let label = match &event.event_type {
                EventType::Birth => "Birthday",
                EventType::Marriage => "Anniversary",
                _ => continue,
            };

            let base_date = match &event.date {
                Some(EventDate::Exact(d)) | Some(EventDate::Approximate(d)) => *d,
                _ => continue,
            };

            // Project the anniversary onto the current or next year
            let anniversary = anniversary_this_or_next_year(base_date, today);

            // Only include if within the window [today, today+days]
            if anniversary >= today && anniversary <= window_end {
                let days_away = (anniversary - today).num_days();
                let name = person.display_name();
                let years = anniversary.year() - base_date.year();
                let description = format!(
                    "{} — {} ({}{})",
                    name,
                    label,
                    format_date_short(anniversary),
                    if years > 0 {
                        format!(", {} yrs", years)
                    } else {
                        String::new()
                    }
                );
                reminders.push((days_away, description, label.to_string(), anniversary));
            }
        }
    }

    reminders.sort_by_key(|(d, _, _, _)| *d);

    if reminders.is_empty() {
        println!(
            "{}",
            format!("No upcoming anniversaries in the next {} day(s).", days).bright_black()
        );
        return Ok(());
    }

    println!(
        "{}\n",
        format!("  Reminders — next {} day(s)  ", days)
            .bold()
            .bright_cyan()
            .on_black()
    );

    for (days_away, description, label, _date) in &reminders {
        let badge = if *days_away == 0 {
            "TODAY ".bright_green().bold().to_string()
        } else if *days_away == 1 {
            "TOMORROW".yellow().bold().to_string()
        } else {
            format!("in {:>3}d ", days_away).bright_black().to_string()
        };
        let label_col = match label.as_str() {
            "Birthday" => label.cyan().to_string(),
            "Anniversary" => label.magenta().to_string(),
            _ => label.normal().to_string(),
        };
        println!("  {} [{}] {}", badge, label_col, description);
    }

    Ok(())
}

/// Given a historical base date, return the nearest upcoming anniversary
/// (this calendar year if still in window, next year otherwise).
fn anniversary_this_or_next_year(base: NaiveDate, today: NaiveDate) -> NaiveDate {
    let this_year = today.year();
    // Try this year's anniversary; if the month/day doesn't exist (e.g. Feb 29),
    // fall back to Mar 1.
    let this = NaiveDate::from_ymd_opt(this_year, base.month(), base.day())
        .or_else(|| NaiveDate::from_ymd_opt(this_year, 3, 1))
        .unwrap_or(today);

    if this >= today {
        this
    } else {
        NaiveDate::from_ymd_opt(this_year + 1, base.month(), base.day())
            .or_else(|| NaiveDate::from_ymd_opt(this_year + 1, 3, 1))
            .unwrap_or(today)
    }
}

fn format_date_short(d: NaiveDate) -> String {
    d.format("%d %b %Y").to_string()
}
