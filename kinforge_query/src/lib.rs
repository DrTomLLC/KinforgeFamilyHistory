use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

/// Simple in-memory query filters applied after loading from storage.
pub struct PersonQuery {
    pub surname_contains: Option<String>,
    pub given_contains: Option<String>,
    pub sex: Option<Sex>,
}

impl PersonQuery {
    pub fn new() -> Self {
        Self {
            surname_contains: None,
            given_contains: None,
            sex: None,
        }
    }

    pub fn surname_contains(mut self, s: impl Into<String>) -> Self {
        self.surname_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn given_contains(mut self, s: impl Into<String>) -> Self {
        self.given_contains = Some(s.into().to_lowercase());
        self
    }

    pub fn sex(mut self, sex: Sex) -> Self {
        self.sex = Some(sex);
        self
    }

    pub fn run(&self, db: &Database) -> KinforgeResult<Vec<Person>> {
        let all = db.list_people()?;
        Ok(all
            .into_iter()
            .filter(|p| self.matches(p))
            .collect())
    }

    fn matches(&self, person: &Person) -> bool {
        if let Some(ref sex) = self.sex {
            if &person.sex != sex {
                return false;
            }
        }
        if let Some(ref surname_filter) = self.surname_contains {
            let found = person.names.iter().any(|n| {
                n.surname
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(surname_filter.as_str()))
                    .unwrap_or(false)
            });
            if !found {
                return false;
            }
        }
        if let Some(ref given_filter) = self.given_contains {
            let found = person.names.iter().any(|n| {
                n.given
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(given_filter.as_str()))
                    .unwrap_or(false)
            });
            if !found {
                return false;
            }
        }
        true
    }
}

impl Default for PersonQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Search all people by name substring (given or surname).
pub fn search_people(db: &Database, query: &str) -> KinforgeResult<Vec<Person>> {
    let q = query.to_lowercase();
    let all = db.list_people()?;
    Ok(all
        .into_iter()
        .filter(|p| {
            p.names.iter().any(|n| {
                let given_match = n
                    .given
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(&q))
                    .unwrap_or(false);
                let surname_match = n
                    .surname
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(&q))
                    .unwrap_or(false);
                given_match || surname_match
            })
        })
        .collect())
}
