use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;
use std::io::Write;

/// Export all data in the database to GEDCOM 5.5 format.
pub fn export_gedcom<W: Write>(db: &Database, writer: &mut W) -> KinforgeResult<()> {
    writeln!(writer, "0 HEAD").ok();
    writeln!(writer, "1 SOUR Kinforge").ok();
    writeln!(writer, "2 VERS 0.1.0").ok();
    writeln!(writer, "1 GEDC").ok();
    writeln!(writer, "2 VERS 5.5").ok();
    writeln!(writer, "1 CHAR UTF-8").ok();

    let people = db.list_people()?;
    for person in &people {
        writeln!(writer, "0 @I{}@ INDI", person.id).ok();

        for name in &person.names {
            let full = format!(
                "{}/{}",
                name.given.as_deref().unwrap_or(""),
                name.surname.as_deref().unwrap_or("")
            );
            writeln!(writer, "1 NAME {}", full).ok();
        }

        let sex_code = match person.sex {
            Sex::Male => "M",
            Sex::Female => "F",
            Sex::Unknown => "U",
        };
        writeln!(writer, "1 SEX {}", sex_code).ok();

        let events = db.list_events_for_person(&person.id)?;
        for event in &events {
            let tag = event_type_to_gedcom_tag(&event.event_type);
            writeln!(writer, "1 {}", tag).ok();

            if let Some(ref date) = event.date {
                let date_str = gedcom_date_str(date);
                writeln!(writer, "2 DATE {}", date_str).ok();
            }

            if let Some(ref place_id) = event.place_id {
                if let Ok(place) = db.get_place(place_id) {
                    writeln!(writer, "2 PLAC {}", place.name).ok();
                }
            }

            if let Some(ref notes) = event.notes {
                writeln!(writer, "2 NOTE {}", notes).ok();
            }
        }

        if let Some(ref notes) = person.notes {
            writeln!(writer, "1 NOTE {}", notes).ok();
        }
    }

    let sources = db.list_sources()?;
    for source in &sources {
        writeln!(writer, "0 @S{}@ SOUR", source.id).ok();
        writeln!(writer, "1 TITL {}", source.title).ok();
        if let Some(ref author) = source.author {
            writeln!(writer, "1 AUTH {}", author).ok();
        }
        if let Some(ref pub_info) = source.publication {
            writeln!(writer, "1 PUBL {}", pub_info).ok();
        }
    }

    writeln!(writer, "0 TRLR").ok();
    Ok(())
}

fn event_type_to_gedcom_tag(et: &EventType) -> &'static str {
    match et {
        EventType::Birth => "BIRT",
        EventType::Death => "DEAT",
        EventType::Marriage => "MARR",
        EventType::Divorce => "DIV",
        EventType::Burial => "BURI",
        EventType::Baptism => "BAPT",
        EventType::Residence => "RESI",
        EventType::Occupation => "OCCU",
        EventType::Education => "EDUC",
        EventType::Military => "MILI",
        EventType::Naturalization => "NATU",
        EventType::Census => "CENS",
        EventType::Emigration => "EMIG",
        EventType::Immigration => "IMMI",
        EventType::Other(_) => "EVEN",
    }
}

fn gedcom_date_str(date: &EventDate) -> String {
    match date {
        EventDate::Exact(d) => d.format("%d %b %Y").to_string().to_uppercase(),
        EventDate::Approximate(d) => format!("ABT {}", d.format("%d %b %Y").to_string().to_uppercase()),
        EventDate::Before(d) => format!("BEF {}", d.format("%d %b %Y").to_string().to_uppercase()),
        EventDate::After(d) => format!("AFT {}", d.format("%d %b %Y").to_string().to_uppercase()),
        EventDate::Between(d1, d2) => format!(
            "BET {} AND {}",
            d1.format("%d %b %Y").to_string().to_uppercase(),
            d2.format("%d %b %Y").to_string().to_uppercase()
        ),
        EventDate::Unknown => "UNKNOWN".to_string(),
    }
}
