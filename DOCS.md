# Kinforge Family History — Project Documentation

> **Last updated:** 2026-03-27
> **Build status:** 90 tests passing · 0 warnings · `cargo build --workspace` clean
> **Version:** 0.9.0

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

Kinforge is a local-first, research-grade genealogy program. It stores all data in a SQLite database file on your own machine — no cloud, no telemetry, no subscription required. It is operated from the command line, or via an interactive terminal TUI.

Key design principles:
- **Local-first**: your data never leaves your machine unless you export it
- **Source-citation discipline**: every fact can be linked to a source record with a confidence rating
- **No data loss**: backups are made automatically on every start; schema migrations preserve all existing data
- **Standard file formats**: import and export GEDCOM 5.5 (the genealogy industry standard) and JSON
- **Research-grade**: full-text search, research task tracking, relationship path finding, narrative reports

---

## Project Status

### Completed and Working

| Area | Status | Tests |
|------|--------|-------|
| Core domain models (Person, Event, Place, Relationship, Source, Citation) | ✅ Complete | — |
| Flexible genealogical dates (exact, approximate, before, after, between) | ✅ Complete | 2 tests |
| SQLite storage with WAL mode and foreign-key constraints | ✅ Complete | 15 unit tests |
| Automatic schema migrations (version-tracked, non-destructive) | ✅ Complete | — |
| Full CRUD for all entity types | ✅ Complete | 24 integration tests |
| Cascade deletes (delete person → events and relationships deleted) | ✅ Complete | tested |
| Validation layer (date ordering, coordinate bounds, self-relationships, etc.) | ✅ Complete | 5 unit tests |
| Backup-on-open system (timestamped `YYYY-MM-DD_HH-MM-SS` copies, auto-prune) | ✅ Complete | 2 unit tests |
| GEDCOM 5.5 export (INDI, FAM, SOUR records) | ✅ Complete | 4 integration tests |
| GEDCOM 5.5 import (people, events, families, sources) | ✅ Complete | 4 integration tests |
| GEDCOM import deduplication (fingerprint-based skip for existing people) | ✅ Complete | tested |
| JSON export and import (full round-trip) | ✅ Complete | — |
| TOML configuration file with XDG-compliant paths | ✅ Complete | 2 unit tests |
| CLI: person add/list/show/update/add-name/delete | ✅ Complete | — |
| CLI: event add/list/show/update/delete with all date qualifiers | ✅ Complete | — |
| CLI: relationship add/show/list/update/delete | ✅ Complete | — |
| CLI: place add/list/show/update/delete with parent hierarchy | ✅ Complete | — |
| CLI: source add/list/show/update/delete | ✅ Complete | — |
| CLI: citation add/list/update/delete | ✅ Complete | — |
| CLI: report stats/people/individual/ancestors/descendants/tree | ✅ Complete | — |
| CLI: search people (by name, sex filter) | ✅ Complete | — |
| CLI: search sources (by title, year range) | ✅ Complete | — |
| CLI: export gedcom/json/csv/html/geojson | ✅ Complete | — |
| CLI: import gedcom/json | ✅ Complete | — |
| CLI: config (show active paths and settings) | ✅ Complete | — |
| ASCII family tree visualization | ✅ Complete | — |
| ASCII ancestor tree visualization | ✅ Complete | — |
| Ancestor report with Ahnentafel numbering | ✅ Complete | — |
| Descendant report with birth years | ✅ Complete | — |
| People list showing birth year | ✅ Complete | — |
| Individual report with life dates summary | ✅ Complete | — |
| Family group sheet report | ✅ Complete | — |
| Chronological timeline report | ✅ Complete | — |
| Narrative prose biography report | ✅ Complete | tested |
| Sources report (with citation counts) | ✅ Complete | tested |
| Place parent hierarchy (town → county → state) | ✅ Complete | 2 tests |
| Relationship notes editing | ✅ Complete | 1 test |
| Query system (PersonQuery, EventQuery, SourceQuery) | ✅ Complete | — |
| Extended relationship types (adoptive, godparent, half-sibling, step, foster) | ✅ Complete | tested |
| Media attachments (photos, documents, audio, video; link to people/events/sources) | ✅ Complete | tested |
| FTS5 full-text search across all entities | ✅ Complete | 4 tests |
| Relationship path finding (BFS; `person path --from X --to Y`) | ✅ Complete | 4 tests |
| Self-contained single-file HTML export | ✅ Complete | tested |
| Duplicate person detection (`kinforge check`) | ✅ Complete | tested |
| Research task tracking (`kinforge task`) | ✅ Complete | 8 tests |
| Upcoming reminders (`kinforge reminders`) | ✅ Complete | — |
| GeoJSON place export (`kinforge export geojson`) | ✅ Complete | — |
| Enhanced statistics (`kinforge report stats --detailed`) | ✅ Complete | — |
| Interactive TUI browser (`kinforge tui`) | ✅ Complete | — |
| Plugin API skeleton | ✅ Skeleton present | — |
| Sync crate skeleton | ✅ Skeleton present | — |

**Total: 90 tests passing, 0 failures, 0 warnings**

---

## Single-User Readiness Checklist

For one person to use Kinforge productively right now:

- [x] Build the binary (`cargo build --release`)
- [x] First run auto-creates the database directory and SQLite file
- [x] Backup fires automatically on every start — no action needed
- [x] Add people, events, relationships, places, sources, citations via CLI
- [x] Use flexible genealogical dates: exact, approximate, before, after, or between
- [x] Link places in a hierarchy (e.g. town → county → state)
- [x] Attach media (photos, documents) to people, events, or sources
- [x] Run reports on individuals, ancestors (with Ahnentafel numbers), descendants, narrative biography
- [x] Track research tasks with priorities, statuses, and person links
- [x] Browse your data interactively with `kinforge tui`
- [x] Search your database by name, notes, or full-text across all entities
- [x] Find the relationship path between any two people
- [x] Export your data to GEDCOM (importable into Ancestry, FamilySearch, etc.)
- [x] Export a self-contained HTML document for sharing
- [x] Export a GeoJSON file for mapping places with coordinates
- [x] Import a GEDCOM from another program to seed your database (duplicates are skipped)
- [x] See upcoming birthdays and anniversaries with `kinforge reminders`
- [x] Run data integrity checks with `kinforge check`
- [x] Override the database path via `--db` flag or `KINFORGE_DB` environment variable
- [x] Configure backup limits via `~/.config/kinforge/config.toml`

**Kinforge is fully usable today as a comprehensive single-user local genealogy tool.**

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

# List everyone (IDs shown are UUIDs; prefix matching works for most commands)
kinforge person list

# Add a birth event (exact date)
kinforge event add --person <UUID-PREFIX> --event-type birth --date 1885-06-15 --place "Boston, MA"

# Add a source
kinforge source add --title "1900 US Census" --author "US Census Bureau" --year 1900

# Link event to source
kinforge citation add --source <SRC-PREFIX> --event <EVT-PREFIX> --page "sheet 12" --confidence primary

# Add a parent-child relationship
kinforge relationship add --person1 <PARENT-PREFIX> --rel-type parent-child --person2 <CHILD-PREFIX>

# Show a full individual report
kinforge report individual <UUID-PREFIX>

# Narrative biography
kinforge report narrative <UUID-PREFIX>

# Find how two people are related
kinforge person path --from <UUID1> --to <UUID2>

# Full-text search across all notes, names, sources
kinforge search fulltext "boston immigration"

# Research tasks
kinforge task add "Find baptism record for John Smith" --priority high --person <UUID-PREFIX>
kinforge task list
kinforge task done <TASK-PREFIX>

# Upcoming birthdays and anniversaries (next 30 days)
kinforge reminders
kinforge reminders --days 60

# Interactive TUI browser
kinforge tui

# See statistics (with histogram and top surnames)
kinforge report stats
kinforge report stats --detailed

# Export
kinforge export gedcom my_family.ged
kinforge export html family.html
kinforge export geojson places.geojson

# Import
kinforge import gedcom existing_family.ged

# Run integrity check
kinforge check
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
├── kinforge.db                          ← your live database
└── backups/
    ├── kinforge_2026-03-20_09-15-00.db
    ├── kinforge_2026-03-21_14-22-31.db
    └── kinforge_2026-03-27_08-00-17.db
```

On Windows the same structure is under `%APPDATA%\kinforge\`.

### Restoring from backup

Close Kinforge, then copy the desired backup file over the live database:

```bash
# Linux/macOS
cp ~/.local/share/kinforge/backups/kinforge_2026-03-21_14-22-31.db \
   ~/.local/share/kinforge/kinforge.db
```

### Configuring backup behaviour

Edit (or create) `~/.config/kinforge/config.toml`:

```toml
backup_on_open = true   # set false to disable automatic backups
max_backups    = 20     # keep up to 20 backup files (oldest pruned)
```

---

## Configuration Reference

Default config file location: `~/.config/kinforge/config.toml` (Linux/macOS) or `%APPDATA%\kinforge\config.toml` (Windows).

| Key | Default | Description |
|-----|---------|-------------|
| `database_path` | `~/.local/share/kinforge/kinforge.db` | Path to the SQLite database |
| `backup_on_open` | `true` | Create a timestamped backup every time the database is opened |
| `max_backups` | `10` | Maximum number of backup files to retain (oldest are pruned) |
| `log_level` | `"warn"` | Log verbosity: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"` |
| `default_export_dir` | _(unset)_ | Default directory for exported files |

---

## Full CLI Command Reference

All commands accept `--db <PATH>` (or `KINFORGE_DB` env var) and `--config <PATH>` (or `KINFORGE_CONFIG` env var) as global flags. Most ID arguments accept a UUID prefix (first 8 characters suffice).

### `person`

```
kinforge person add --given <NAME> --surname <NAME> --sex <male|female|unknown> [--notes <TEXT>]
kinforge person list
kinforge person show <ID>
kinforge person update <ID> [--sex <SEX>] [--notes <TEXT>]
kinforge person add-name <ID> [--given <NAME>] [--surname <NAME>] [--name-type <birth|married|aka|other>]
kinforge person delete <ID>
kinforge person add-adoptive-parent --child <ID> --parent <ID>
kinforge person add-godparent --godchild <ID> --godparent <ID>
kinforge person path --from <ID> --to <ID>
```

### `event`

```
kinforge event add --person <ID> --event-type <TYPE> [--date <DATE>] [--qualifier <QUAL>] [--date2 <DATE>] [--place <NAME>] [--notes <TEXT>]
kinforge event list <PERSON-ID>
kinforge event show <ID>
kinforge event update <ID> [--date <DATE>] [--qualifier <QUAL>] [--date2 <DATE>] [--notes <TEXT>]
kinforge event delete <ID>
```

**Date formats:** `YYYY-MM-DD`, `YYYY-MM` (1st of month), `YYYY` (Jan 1)

**Qualifiers:** `exact` (default) · `approximate`/`abt` · `before`/`bef` · `after`/`aft` · `between`/`bet` (requires `--date2`)

**Event types:** `birth` · `death` · `marriage` · `divorce` · `burial` · `baptism` · `residence` · `occupation` · `education` · `military` · `naturalization` · `census` · `emigration` · `immigration` · any custom string

### `relationship`

```
kinforge relationship add --person1 <ID> --rel-type <TYPE> --person2 <ID> [--notes <TEXT>]
kinforge relationship show <ID>
kinforge relationship list <PERSON-ID>
kinforge relationship update <ID> [--notes <TEXT>]
kinforge relationship delete <ID>
```

**Relationship types:** `parent-child` · `spouse` · `sibling` · `adoptive-parent` · `godparent` · `half-sibling` · `step-parent` · `foster`

For `parent-child`/`adoptive-parent`/`step-parent`/`foster`: `--person1` is the **parent** role, `--person2` is the **child** role.

### `place`

```
kinforge place add --name <NAME> [--lat <DECIMAL>] [--lon <DECIMAL>] [--parent <ID>]
kinforge place list
kinforge place show <ID>
kinforge place update <ID> [--name <NAME>] [--lat <DECIMAL>] [--lon <DECIMAL>] [--parent <ID>]
kinforge place delete <ID>
```

### `source`

```
kinforge source add --title <TITLE> [--author <NAME>] [--publication <TEXT>] [--year <YEAR>] [--notes <TEXT>]
kinforge source list
kinforge source show <ID>
kinforge source update <ID> [--title <T>] [--author <A>] [--year <Y>] [--notes <N>]
kinforge source delete <ID>
```

### `citation`

```
kinforge citation add --source <ID> --event <ID> [--page <TEXT>] --confidence <LEVEL> [--notes <TEXT>]
kinforge citation list --event <ID>
kinforge citation update <ID> [--page <TEXT>] [--confidence <LEVEL>] [--notes <TEXT>]
kinforge citation delete <ID>
```

**Confidence levels:** `unreliable` · `questionable` · `secondary` · `primary` · `direct`

### `media`

```
kinforge media add --title <TITLE> --type <photo|document|audio|video|other> [--path <FILE>] [--description <TEXT>]
kinforge media list
kinforge media show <ID>
kinforge media update <ID> [--title <T>] [--description <D>]
kinforge media delete <ID>
kinforge media attach <MEDIA-ID> --entity-type <person|event|source> --entity-id <ID>
kinforge media detach <LINK-ID>
kinforge media for-person <PERSON-ID>
kinforge media for-event <EVENT-ID>
```

### `task`

```
kinforge task add <DESCRIPTION> [--priority <low|medium|high>] [--person <ID>] [--notes <TEXT>]
kinforge task list [--status <pending|in-progress|done>] [--priority <low|medium|high>] [--person <ID>]
kinforge task show <ID>
kinforge task update <ID> [--description <D>] [--priority <P>] [--status <S>] [--notes <N>] [--person <ID>] [--clear-person]
kinforge task start <ID>
kinforge task done <ID>
kinforge task delete <ID>
```

### `report`

```
kinforge report stats [--detailed]
kinforge report people
kinforge report individual <ID>
kinforge report ancestors <ID> [--generations <N>]
kinforge report descendants <ID> [--generations <N>]
kinforge report tree <ID> [--depth <N>]
kinforge report ancestor-tree <ID> [--depth <N>]
kinforge report family <ID>
kinforge report timeline <ID>
kinforge report sources
kinforge report narrative <ID>
```

`--detailed` adds a birth-decade histogram and top-10 surname frequency table.

### `search`

```
kinforge search people <QUERY> [--sex <male|female|unknown>]
kinforge search sources <QUERY> [--from-year <YEAR>] [--to-year <YEAR>]
kinforge search fulltext <QUERY>
```

### `export`

```
kinforge export gedcom <OUTPUT>
kinforge export json <OUTPUT>
kinforge export csv <OUTPUT>
kinforge export html <OUTPUT>
kinforge export geojson <OUTPUT>
```

### `import`

```
kinforge import gedcom <INPUT>
kinforge import json <INPUT>
```

Import is additive — existing records are not deleted. GEDCOM import skips people with the same name + birth year already in the database. Both GEDCOM and JSON imports print a summary of records added (people, events, sources, relationships, places).

### `reminders`

```
kinforge reminders [--days <N>]     # default: 30 days ahead
```

Shows upcoming birthdays (Birth events) and anniversaries (Marriage events).

### `check`

```
kinforge check
```

Reports duplicate people (same name), orphaned citations, and other integrity issues.

### `config`

```
kinforge config show
kinforge config init
kinforge config set <KEY> <VALUE>
```

### `tui`

```
kinforge tui
```

Opens an interactive terminal UI with four tabs:

| Tab | Content |
|-----|---------|
| **People** | Scrollable list with birth years; `/` to filter by name; `Enter` for detail panel showing events and relationships |
| **Tasks** | Scrollable list grouped by status (In Progress → Pending → Done); shows priority badge and strikethrough for done tasks |
| **Sources** | Scrollable list with citation counts; `Enter` for citation detail panel showing linked events |
| **Stats** | Database record counts and database file path |

**Key bindings:**

| Key | Context | Effect |
|-----|---------|--------|
| `Tab` / `Shift-Tab` | Any | Switch tabs |
| `↑` / `↓` or `k` / `j` | Any | Navigate list or scroll detail panel |
| `n` | People | Open inline popup to create a new person |
| `/` | People | Enter search/filter mode |
| `Esc` | Search / detail panel | Cancel search or close detail panel |
| `Enter` | People, Sources | Open detail panel; close if already open |
| `d` / `c` | Tasks | Mark selected task as Done |
| `n` | Tasks | Open inline input to create a new task |
| `p` | Tasks | Cycle selected task's priority (Low → Medium → High → Low) |
| `x` | Tasks | Delete selected task |
| `q` or `Ctrl+C` | Any | Quit |

---

## Data Safety and Migration Policy

### No data loss on upgrades

Kinforge uses a `schema_version` table in the database. On every open, it checks the current schema version and applies any pending migrations in order. Migrations are append-only — they add new columns or tables but never drop existing data.

Currently at schema version 4 (people, events, places, relationships, sources, citations, media, FTS index, research tasks).

### Backup guarantee

With default settings (`backup_on_open = true`, `max_backups = 10`), you have up to 10 timestamped snapshots. Increase `max_backups` for longer retention.

### Export as an additional safety net

Use `kinforge export json family.json` before any major import or batch operation. The JSON file is human-readable and can be re-imported if something goes wrong.

### SQLite WAL mode

The database runs in Write-Ahead Logging (WAL) mode for better concurrency and crash safety.

---

## Architecture Overview

```
kinforge_core          — domain models, types, error type, validation
kinforge_storage       — SQLite persistence (rusqlite), schema migrations (v1–v4), FTS5
kinforge_config        — TOML config file, XDG path resolution
kinforge_query         — fluent query builders (PersonQuery, EventQuery, SourceQuery)
kinforge_import_export — GEDCOM 5.5 parser/writer, JSON import/export
kinforge_reports       — text report generators (individual, ancestors, narrative, HTML)
kinforge_viz           — ASCII tree renderers (family tree, ancestor tree)
kinforge_app           — Application facade: CRUD, search, tasks, path-finding, integrity
kinforge_cli           — clap CLI (binary) + ratatui TUI
kinforge_plugin_api    — plugin trait skeleton (future extensibility)
kinforge_sync          — sync/replication skeleton (not yet implemented)
kinforge_ui_desktop    — desktop GUI skeleton (not yet implemented)
```

### Schema versions

| Version | Tables added |
|---------|-------------|
| 1 | `people`, `person_names`, `places`, `events`, `relationships`, `sources`, `citations` |
| 2 | `media`, `media_links` |
| 3 | `fts_index` (FTS5 virtual table) |
| 4 | `research_tasks` |

---

## What Is Not Yet Implemented

| Feature | Notes |
|---------|-------|
| Desktop GUI | `kinforge_ui_desktop` stub only; no UI framework chosen |
| Cloud/peer sync | `kinforge_sync` stub only |
| Plugin loading | Trait defined; no loader or host |
| In-place name editing | `person update-name` / `person delete-name` commands available |

---

## Roadmap

**Completed (recent):**
- [x] `kinforge person update-name` / `delete-name` — edit name entries in place
- [x] `kinforge_app::db` fully private — all access via `Application::database()` accessor
- [x] TUI: Sources tab with citation detail panel
- [x] TUI: task quick-complete (`d`/`c`), new task (`n`), priority cycle (`p`), delete (`x`)

**Near-term:**
- [x] JSON import statistics (people, events, sources, relationships, places counts)
- [x] TUI: inline person creation (People tab, `n` key, two-field popup)

**Long-term:**
- [ ] Desktop GUI (`kinforge_ui_desktop`)
- [ ] Optional sync between devices (`kinforge_sync`)
- [ ] Plugin loading at runtime (`kinforge_plugin_api`)
