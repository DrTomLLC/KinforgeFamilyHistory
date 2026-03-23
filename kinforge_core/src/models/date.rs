use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventDate {
    Exact(NaiveDate),
    Approximate(NaiveDate),
    Before(NaiveDate),
    After(NaiveDate),
    Between(NaiveDate, NaiveDate),
    Unknown,
}

impl EventDate {
    pub fn kind_str(&self) -> &'static str {
        match self {
            EventDate::Exact(_) => "exact",
            EventDate::Approximate(_) => "approximate",
            EventDate::Before(_) => "before",
            EventDate::After(_) => "after",
            EventDate::Between(_, _) => "between",
            EventDate::Unknown => "unknown",
        }
    }

    pub fn date_str(&self) -> Option<String> {
        match self {
            EventDate::Exact(d) | EventDate::Approximate(d) | EventDate::Before(d) | EventDate::After(d) => {
                Some(d.format("%Y-%m-%d").to_string())
            }
            EventDate::Between(d1, _) => Some(d1.format("%Y-%m-%d").to_string()),
            EventDate::Unknown => None,
        }
    }

    pub fn date2_str(&self) -> Option<String> {
        match self {
            EventDate::Between(_, d2) => Some(d2.format("%Y-%m-%d").to_string()),
            _ => None,
        }
    }

    pub fn from_parts(
        kind: &str,
        date_val: Option<&str>,
        date_val2: Option<&str>,
    ) -> Option<Self> {
        let parse = |s: &str| -> Option<NaiveDate> {
            NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
        };

        match kind {
            "exact" => parse(date_val?)
                .map(EventDate::Exact),
            "approximate" => parse(date_val?)
                .map(EventDate::Approximate),
            "before" => parse(date_val?)
                .map(EventDate::Before),
            "after" => parse(date_val?)
                .map(EventDate::After),
            "between" => {
                let d1 = parse(date_val?)?;
                let d2 = parse(date_val2?)?;
                Some(EventDate::Between(d1, d2))
            }
            "unknown" | "" => Some(EventDate::Unknown),
            _ => None,
        }
    }
}

impl fmt::Display for EventDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventDate::Exact(d) => write!(f, "{}", d.format("%d %b %Y")),
            EventDate::Approximate(d) => write!(f, "about {}", d.format("%d %b %Y")),
            EventDate::Before(d) => write!(f, "before {}", d.format("%d %b %Y")),
            EventDate::After(d) => write!(f, "after {}", d.format("%d %b %Y")),
            EventDate::Between(d1, d2) => {
                write!(f, "between {} and {}", d1.format("%d %b %Y"), d2.format("%d %b %Y"))
            }
            EventDate::Unknown => write!(f, "unknown"),
        }
    }
}
