use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(MIGRATION_1)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    if version < 2 {
        conn.execute_batch(MIGRATION_2)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
    }

    if version < 3 {
        conn.execute_batch(MIGRATION_3)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
    }

    Ok(())
}

const MIGRATION_1: &str = "
CREATE TABLE IF NOT EXISTS people (
    id      TEXT PRIMARY KEY,
    sex     TEXT NOT NULL DEFAULT 'Unknown',
    notes   TEXT
);

CREATE TABLE IF NOT EXISTS person_names (
    id          TEXT PRIMARY KEY,
    person_id   TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    name_type   TEXT NOT NULL DEFAULT 'Birth',
    given       TEXT,
    surname     TEXT,
    prefix      TEXT,
    suffix      TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS places (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    latitude    REAL,
    longitude   REAL,
    parent_id   TEXT REFERENCES places(id)
);

CREATE TABLE IF NOT EXISTS events (
    id          TEXT PRIMARY KEY,
    person_id   TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    event_type  TEXT NOT NULL,
    date_kind   TEXT,
    date_value  TEXT,
    date_value2 TEXT,
    place_id    TEXT REFERENCES places(id),
    notes       TEXT
);

CREATE TABLE IF NOT EXISTS relationships (
    id          TEXT PRIMARY KEY,
    rel_type    TEXT NOT NULL,
    person1_id  TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    person2_id  TEXT NOT NULL REFERENCES people(id) ON DELETE CASCADE,
    notes       TEXT
);

CREATE TABLE IF NOT EXISTS sources (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    author      TEXT,
    publication TEXT,
    year        INTEGER,
    repository  TEXT,
    notes       TEXT
);

CREATE TABLE IF NOT EXISTS citations (
    id          TEXT PRIMARY KEY,
    source_id   TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    event_id    TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    page        TEXT,
    confidence  TEXT NOT NULL DEFAULT 'Secondary',
    notes       TEXT
);

CREATE INDEX IF NOT EXISTS idx_person_names_person_id ON person_names(person_id);
CREATE INDEX IF NOT EXISTS idx_events_person_id ON events(person_id);
CREATE INDEX IF NOT EXISTS idx_relationships_person1 ON relationships(person1_id);
CREATE INDEX IF NOT EXISTS idx_relationships_person2 ON relationships(person2_id);
CREATE INDEX IF NOT EXISTS idx_citations_event_id ON citations(event_id);
CREATE INDEX IF NOT EXISTS idx_citations_source_id ON citations(source_id);
";

const MIGRATION_2: &str = "
CREATE TABLE IF NOT EXISTS media (
    id          TEXT PRIMARY KEY,
    title       TEXT NOT NULL,
    path        TEXT,
    url         TEXT,
    media_type  TEXT NOT NULL DEFAULT 'Other',
    description TEXT,
    date        TEXT
);

CREATE TABLE IF NOT EXISTS media_links (
    id          TEXT PRIMARY KEY,
    media_id    TEXT NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_id   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_media_links_media_id ON media_links(media_id);
CREATE INDEX IF NOT EXISTS idx_media_links_entity ON media_links(entity_type, entity_id);
";

const MIGRATION_3: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS fts_index USING fts5(
    body,
    entity_type UNINDEXED,
    entity_id   UNINDEXED,
    tokenize    = 'unicode61'
);
";
