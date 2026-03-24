# Kinforge Family History — Project Documentation

> **Last updated:** 2026-03-24
> **Build status:** 43/43 tests passing · 0 clippy warnings · `cargo fmt` clean

---

## Table of Contents

1. [What Is Kinforge?](#what-is-kinforge)
2. [Project Status](#project-status)
3. [Single-User Readiness Checklist](#single-user-readiness-checklist)
4. [How to Build and Install](#how-to-build-and-install)
5. [How to Run Kinforge](#how-to-run-kinforge)
6. [Backup System](#backup-system)
7. [Configuration Reference](#configuration-reference)
8. [Full CLI Command Reference](#full-cli-command-reference)
9. [Data Safety and Migration Policy](#data-safety-and-migration-policy)
10. [Architecture Overview](#architecture-overview)
11. [What Is Not Yet Implemented](#what-is-not-yet-implemented)
12. [Roadmap](#roadmap)

---

## What Is Kinforge?

Kinforge is a local-first, research-grade genealogy program. It stores all data in a SQLite database file on your own machine — no cloud, no telemetry, no subscription required. It is operated from the command line.

Key design principles:
- **Local-first**: your data never leaves your machine unless you export it
- **Source-citation discipline**: every fact can be linked to a source record with a confidence rating
- **No data loss**: backups are made automatically on every start; schema migrations preserve all existing data
- **Standard file formats**: import and export GEDCOM 5.5 (the genealogy industry standard) and JSON

---

## Project Status

### Completed and Working

| Area | Status | Tests |
|------|--------|-------|
| Core domain models (Person, Event, Place, Relationship, Source, Citation) | ✅ Complete | — |
| Flexible genealogical dates (exact, approximate, before, after, between) | ✅ Complete | — |
| SQLite storage with WAL mode and foreign-key constraints | ✅ Complete | 15 unit tests |
| Automatic schema migrations (version-tracked, non-destructive) | ✅ Complete | — |
| Full CRUD for all entity types | ✅ Complete | 17 integration tests |
| Cascade deletes (delete person → events and relationships deleted) | ✅ Complete | tested |
| Validation layer (date ordering, coordinate bounds, self-relationships, etc.) | ✅ Complete | 5 unit tests |
| Backup-on-open system (timestamped copies, configurable max count, auto-prune) | ✅ Complete | 2 unit tests |
| GEDCOM 5.5 export (INDI, FAM, SOUR records) | ✅ Complete | 4 integration tests |
| GEDCOM 5.5 import (people, events, families, sources) | ✅ Complete | 4 integration tests |
| JSON export and import (full round-trip) | ✅ Complete | — |
| TOML configuration file with XDG-compliant paths | ✅ Complete | 2 unit tests |
| CLI: person add/list/show/update/add-name/delete | ✅ Complete | — |
| CLI: event add/list/show/update/delete | ✅ Complete | — |
| CLI: relationship add/list/delete | ✅ Complete | — |
| CLI: place add/list/show/update/delete | ✅ Complete | — |
| CLI: source add/list/show/update/delete | ✅ Complete | — |
| CLI: citation add/list/update/delete | ✅ Complete | — |
| CLI: report stats/people/individual/ancestors/descendants/tree | ✅ Complete | — |
| CLI: search people (by name, sex filter) | ✅ Complete | — |
| CLI: search sources (by title, year range) | ✅ Complete | — |
| CLI: export gedcom/json | ✅ Complete | — |
| CLI: import gedcom/json | ✅ Complete | — |
| CLI: config (show active paths and settings) | ✅ Complete | — |
| ASCII family tree visualization | ✅ Complete | — |
| Ancestor and descendant reports | ✅ Complete | — |
| Query system (PersonQuery, EventQuery, SourceQuery) | ✅ Complete | — |
| Plugin API skeleton | ✅ Skeleton present | — |
| Sync crate skeleton | ✅ Skeleton present | — |

**Total: 43 tests passing, 0 failures, 0 clippy warnings**

---

## Single-User Readiness Checklist

For one person to use Kinforge productively right now:

- [x] Build the binary (`cargo build --release`)
- [x] First run auto-creates the database directory and SQLite file
- [x] Backup fires automatically on every start — no action needed
- [x] Add people, events, relationships, places, sources, citations via CLI
- [x] Run reports on individuals, ancestors, descendants
- [x] Export your data to GEDCOM (importable into Ancestry, FamilySearch, etc.)
- [x] Import a GEDCOM from another program to seed your database
- [x] Search your database by name, sex, or source title
- [x] Override the database path via `--db` flag or `KINFORGE_DB` environment variable
- [x] Configure backup limits via `~/.config/kinforge/config.toml`

**Kinforge is usable today as a single-user CLI genealogy tool with backup protection.**

The one thing to be aware of: there is no interactive editing yet (you cannot edit an existing name's spelling directly; you delete and re-add). That is a known limitation tracked in the roadmap.

---

## How to Build and Install

### Prerequisites

- Rust toolchain 1.75 or newer (install via https://rustup.rs)
- No system dependencies — SQLite is bundled via the `rusqlite` "bundled" feature

### Build a release binary

```bash
git clone <repo-url>
cd KinforgeFamilyHistory
cargo build --release
```

The binary is at `target/release/kinforge` (Linux/macOS) or `target\release\kinforge.exe` (Windows).

### Optional: install to your PATH

```bash
cargo install --path kinforge_cli
```

Or copy the binary manually:

```bash
# Linux/macOS
cp target/release/kinforge ~/.local/bin/

# Windows (PowerShell)
Copy-Item target\release\kinforge.exe $env:USERPROFILE\bin\
```

### Run without installing

```bash
./target/release/kinforge --help
# or during development:
cargo run --bin kinforge -- --help
```

---

## How to Run Kinforge

On first run, Kinforge automatically:
1. Creates `~/.local/share/kinforge/` (Linux/macOS) or `%APPDATA%\kinforge\` (Windows)
2. Creates `kinforge.db` inside that directory
3. Applies the database schema
4. Creates a timestamped backup (if the database already exists and `backup_on_open = true`)

### Quick start example

```bash
# Add some people
kinforge person add --given "John" --surname "Smith" --sex male
kinforge person add --given "Mary" --surname "Jones" --sex female

# See who's in the database (IDs are UUIDs like a1b2c3d4-...)
kinforge person list

# Add a birth event (use the UUID shown in person list)
kinforge event add --person <UUID> --event-type birth --date 1885-06-15 --place "Boston, MA"

# Add a source
kinforge source add --title "1900 US Census" --author "US Census Bureau" --year 1900

# Link event to source
kinforge citation add --source <SOURCE-UUID> --event <EVENT-UUID> --page "sheet 12" --confidence primary

# Show a full individual report
kinforge report individual <UUID>

# See statistics
kinforge report stats

# Export to GEDCOM (for use in other software)
kinforge export gedcom my_family.ged

# Import a GEDCOM file
kinforge import gedcom existing_family.ged
```

---

## Backup System

### How it works

Every time Kinforge opens a database, it checks whether `backup_on_open = true` (the default). If so, it:

1. Copies the existing `.db` file to a `backups/` subdirectory next to the database
2. Names the copy `kinforge_<YYYY-MM-DD_HH-MM-SS>.db`
3. Counts how many backup files exist; if there are more than `max_backups` (default: 10), the oldest files are deleted

No backup is created if the database does not yet exist (first run).

### Where backups are stored

```
~/.local/share/kinforge/
├── kinforge.db              ← your live database
└── backups/
    ├── kinforge_2026-03-20_09-15-00.db
    ├── kinforge_2026-03-21_14-22-31.db
    └── kinforge_2026-03-24_08-00-17.db
```

On Windows the same structure is under `%APPDATA%\kinforge\`.

### Restoring from backup

Close Kinforge, then copy the desired backup file over the live database:

```bash
# Linux/macOS
cp ~/.local/share/kinforge/backups/kinforge_2026-03-21_14-22-31.db \
   ~/.local/share/kinforge/kinforge.db

# Windows (PowerShell)
Copy-Item "$env:APPDATA\kinforge\backups\kinforge_2026-03-21_14-22-31.db" `
          "$env:APPDATA\kinforge\kinforge.db"
```

### Configuring backup behaviour

Edit (or create) `~/.config/kinforge/config.toml`:

```toml
backup_on_open = true   # set false to disable automatic backups
max_backups    = 20     # keep up to 20 backup files (oldest pruned)
```

### Using a different database file

```bash
# Via flag (overrides config)
kinforge --db /path/to/myproject.db person list

# Via environment variable
export KINFORGE_DB=/path/to/myproject.db
kinforge person list
```

This lets you keep separate databases for different research projects.

---

## Configuration Reference

Default config file location: `~/.config/kinforge/config.toml` (Linux/macOS) or `%APPDATA%\kinforge\config.toml` (Windows).

If the file does not exist, all defaults apply. The file is standard TOML.

| Key | Default | Description |
|-----|---------|-------------|
| `database_path` | `~/.local/share/kinforge/kinforge.db` | Path to the SQLite database |
| `backup_on_open` | `true` | Create a timestamped backup every time the database is opened |
| `max_backups` | `10` | Maximum number of backup files to retain (oldest are pruned) |
| `log_level` | `"warn"` | Log verbosity: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"` |
| `default_export_dir` | _(unset)_ | Default directory for exported files |

Example full config file:

```toml
database_path    = "/home/alice/genealogy/family.db"
backup_on_open   = true
max_backups      = 30
log_level        = "warn"
default_export_dir = "/home/alice/genealogy/exports"
```

View current active configuration:

```bash
kinforge config
```

---

## Full CLI Command Reference

All commands accept `--db <PATH>` (or `KINFORGE_DB` env var) and `--config <PATH>` (or `KINFORGE_CONFIG` env var) as global flags.

### `person`

```
kinforge person add --given <NAME> --surname <NAME> --sex <male|female|unknown> [--notes <TEXT>]
kinforge person list
kinforge person show <UUID>
kinforge person update <UUID> [--sex <SEX>] [--notes <TEXT>]
kinforge person add-name <UUID> [--given <NAME>] [--surname <NAME>] [--name-type <birth|married|aka|other>]
kinforge person delete <UUID>
```

### `event`

```
kinforge event add --person <UUID> --event-type <TYPE> [--date <YYYY-MM-DD>] [--place <NAME>] [--notes <TEXT>]
kinforge event list <PERSON-UUID>
kinforge event show <UUID>
kinforge event update <UUID> [--date <YYYY-MM-DD>] [--notes <TEXT>]
kinforge event delete <UUID>
```

Event types: `birth`, `death`, `marriage`, `divorce`, `burial`, `baptism`, `residence`, `occupation`, `education`, `military`, `naturalization`, `census`, `emigration`, `immigration`, or any custom string.

### `relationship`

```
kinforge relationship add --person1 <UUID> --person2 <UUID> --rel-type <TYPE> [--notes <TEXT>]
kinforge relationship list --person <UUID>
kinforge relationship delete <UUID>
```

Relationship types: `spouse`, `parent-child`, `sibling`, `other`.

### `place`

```
kinforge place add <NAME> [--lat <DECIMAL>] [--lon <DECIMAL>]
kinforge place list
kinforge place show <UUID>
kinforge place update <UUID> [--name <NAME>] [--lat <DECIMAL>] [--lon <DECIMAL>]
kinforge place delete <UUID>
```

Latitude must be −90..90; longitude must be −180..180. Validation rejects out-of-range values.

### `source`

```
kinforge source add --title <TITLE> [--author <NAME>] [--publication <TEXT>] [--year <YEAR>] [--notes <TEXT>]
kinforge source list
kinforge source show <UUID>
kinforge source update <UUID> [--title <T>] [--author <A>] [--year <Y>] [--notes <N>]
kinforge source delete <UUID>
```

### `citation`

```
kinforge citation add --source <UUID> --event <UUID> [--page <TEXT>] --confidence <LEVEL> [--notes <TEXT>]
kinforge citation list --event <UUID>
kinforge citation update <UUID> [--page <TEXT>] [--confidence <LEVEL>] [--notes <TEXT>]
kinforge citation delete <UUID>
```

Confidence levels: `unreliable`, `questionable`, `secondary`, `primary`, `direct`.

### `report`

```
kinforge report stats
kinforge report people
kinforge report individual <UUID>
kinforge report ancestors <UUID> [--generations <N>]        # default: 4
kinforge report descendants <UUID> [--generations <N>]     # default: 4
kinforge report tree <UUID> [--depth <N>]                  # ASCII tree, default depth: 3
```

### `search`

```
kinforge search people <QUERY> [--sex <male|female|unknown>]
kinforge search sources <QUERY> [--from-year <YEAR>] [--to-year <YEAR>]
```

The `QUERY` string is matched case-insensitively against all name parts (people) or title/author (sources).

### `export`

```
kinforge export gedcom <OUTPUT-FILE>
kinforge export json <OUTPUT-FILE>
```

GEDCOM exports INDI (individuals), FAM (families), and SOUR (sources) records. JSON export includes all entities.

### `import`

```
kinforge import gedcom <INPUT-FILE>
kinforge import json <INPUT-FILE>
```

Import is additive — existing records are not deleted. Importing a GEDCOM twice will create duplicates; use with care on a fresh database or after inspecting the file.

### `config`

```
kinforge config
```

Prints the active database path, config file path, backup settings, and log level.

---

## Data Safety and Migration Policy

### No data loss on upgrades

Kinforge uses a `schema_version` table in the database. On every open, it checks the current schema version and applies any pending migrations in order. Migrations are append-only — they add new columns or tables but never drop existing data.

If you upgrade to a newer version of Kinforge with a newer schema, your data is automatically migrated forward. Downgrading is not supported, but the backup system means you can always roll back.

### Backup guarantee

With default settings (`backup_on_open = true`, `max_backups = 10`), you have up to 10 daily snapshots. As long as you run Kinforge at least once every 10 days, you always have a full history going back 10 runs. Increase `max_backups` if you want a longer history.

### Export as an additional safety net

Use `kinforge export json family.json` before any major import or batch operation. The JSON file is human-readable and can be re-imported if something goes wrong.

### SQLite WAL mode

The database runs in Write-Ahead Logging (WAL) mode for better concurrency and crash safety. In the unlikely event of a power loss mid-write, SQLite's WAL ensures the database remains consistent.

---

## Architecture Overview

```
kinforge_core          — domain models, types, error type, validation
kinforge_storage       — SQLite persistence (rusqlite), schema migrations
kinforge_config        — TOML config file, XDG path resolution
kinforge_query         — in-memory query builder over storage
kinforge_import_export — GEDCOM 5.5 parser/writer, JSON import/export
kinforge_reports       — text report generators (individual, ancestors, etc.)
kinforge_viz           — ASCII tree renderer
kinforge_app           — high-level Application facade (used by CLI and tests)
kinforge_cli           — clap-based command-line interface (the binary)
kinforge_plugin_api    — plugin trait skeleton (future extensibility)
kinforge_sync          — sync/replication skeleton (not yet implemented)
kinforge_ui_desktop    — desktop GUI skeleton (not yet implemented)
```

The CLI only depends on `kinforge_app`. Tests in `kinforge_app` use an in-memory SQLite database for speed and isolation.

---

## What Is Not Yet Implemented

The following features are planned but not yet built:

| Feature | Crate | Notes |
|---------|-------|-------|
| Desktop GUI | `kinforge_ui_desktop` | Skeleton only; no UI framework chosen |
| Cloud/peer sync | `kinforge_sync` | Skeleton only; designed for future addition |
| Plugin loading | `kinforge_plugin_api` | Trait defined; no loader or host yet |
| In-place name editing | `kinforge_app` / CLI | Must delete and re-add name entries |
| Place hierarchy (county → state → country) | `kinforge_core` | Parent field exists in DB; not exposed in CLI |
| Relationship notes editing | CLI | Notes stored but not editable via CLI |
| Duplicate detection on GEDCOM import | `kinforge_import_export` | Import is purely additive |
| Interactive date entry (ABT, BEF, AFT, BET) via CLI | CLI | Only exact `YYYY-MM-DD` dates accepted in CLI |
| Note editing for existing records | CLI | Notes can only be set on create or full update |
| Media/document attachments | `kinforge_core` | Not in schema |
| DNA match integration | — | Not planned in current scope |

---

## Roadmap

**Near-term (next iteration):**
- [ ] Expose place parent hierarchy in CLI (`place add --parent <UUID>`)
- [ ] CLI support for all `EventDate` variants (`--date "ABT 1890"`, `--date "BEF 1900"`, `--date "BET 1880 AND 1885"`)
- [ ] Relationship notes editing (`relationship update <UUID> --notes <TEXT>`)
- [ ] Duplicate detection / merge on GEDCOM import
- [ ] `kinforge_app` public field `db` should be private (expose only via methods)

**Medium-term:**
- [ ] Interactive TUI (terminal user interface) using `ratatui`
- [ ] Additional report formats: Ahnentafel chart, family group sheet
- [ ] Full-text notes search
- [ ] Media attachment support (link filenames to records)

**Long-term:**
- [ ] Desktop GUI (`kinforge_ui_desktop`)
- [ ] Optional sync between devices via a self-hosted server (`kinforge_sync`)
- [ ] Plugin loading at runtime (`kinforge_plugin_api`)
