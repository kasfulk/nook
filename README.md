<p align="center">
  <img src="docs/screenshots/data-grid.png" alt="Nook showing the customers table in the data grid" width="960">
</p>

<h1 align="center">Nook</h1>

<p align="center">
  <strong>Database administration for SQLite, PostgreSQL and MySQL — native, fast, and drivable by agents.</strong><br>
  A Navicat-style workbench in one Rust codebase: a desktop app, a CLI, and an MCP server.
</p>

<p align="center">
  <a href="https://github.com/kasfulk/nook/actions/workflows/ci.yml"><img src="https://github.com/kasfulk/nook/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <img src="https://img.shields.io/badge/rust-1.95%2B-orange" alt="Rust 1.95+">
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="MIT OR Apache-2.0">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey" alt="macOS, Linux and Windows">
</p>

Nook is a database workbench that stays out of the way. Open a SQLite file, a PostgreSQL server or a
MySQL database, browse it, edit rows, write SQL, design tables, move data between engines, back it up,
schedule the boring parts, and let an agent drive the same commands — without a browser, an account,
or a cloud round trip.

Connections, browsing, data grid, SQL editor, design tools, transfer, sync, backup, scheduler,
monitor, privileges, reports, ER diagrams and test-data generation: every action is a registered
command, shared by all three surfaces.

## A look inside

| Query editor with live results | Server monitor |
|---|---|
| ![Query editor](docs/screenshots/query-editor.png) | ![Server monitor](docs/screenshots/monitor.png) |

## Quick start

Requires Rust 1.95 or newer (`rustup`). macOS is fully supported; Linux and Windows build from the
same source and are covered by the CI matrix. `ssh` is needed for tunneled connections, and
`pg_dump`/`mysqldump` only for those backup types.

```sh
git clone https://github.com/kasfulk/nook.git
cd nook
./install.sh          # builds the release binaries into ~/.local/bin

nook --demo           # the desktop app, on a sample SQLite database
nook-cli help         # the same surface from a shell
```

`PREFIX=/usr/local ./install.sh` installs elsewhere. To build without installing:

```sh
cargo build --release -p nook -p nook-cli   # binaries land in target/release/
cargo run -p nook -- --demo
```

## What you get

| Area | What you get |
|---|---|
| **Connections** | SQLite files, PostgreSQL, MySQL/MariaDB; SSH tunnel through the system `ssh`; SSL modes (`disable`/`prefer`/`require`/`verify-ca`/`verify-full`); read-only profiles; passwords in the OS keychain (`security` on macOS, `secret-tool` on Linux); connection groups; saved profiles in JSON |
| **Browse** | Catalog read in one pass: schemas, tables, views, columns, types, defaults, comments, indexes, foreign keys, row estimates and sizes |
| **Data grid** | Paging, sort by any column, `WHERE` filter, inline cell editing with type coercion, insert/duplicate/delete/`NULL`, a pending-changes list showing the exact statements, one-transaction apply with rollback on first failure |
| **Query editor** | Multi-statement scripts, run statement at cursor, formatter, completion from the live catalog, `EXPLAIN`/`EXPLAIN ANALYZE`, history, snippets, transactions |
| **AI** | Natural-language prompt to SQL across 7 providers — OpenAI, Anthropic, DeepSeek, Kimi, GLM, Ollama, vLLM — each with its own base URL; `@table` in the prompt expands to that table's columns, keys and indexes; API keys in the login keychain, never in the profile file |
| **Design** | Create, copy, rename, drop tables; add/drop/rename/retype columns; nullability and defaults; indexes; foreign keys; views; object DDL; object search |
| **Transfer** | Copy tables between any two connections, across engines (SQLite ↔ PostgreSQL ↔ MySQL), with page-batched transactions and a per-table report |
| **Synchronize** | Row-level data sync by primary key (dry run first), structure comparison that emits DDL, structure sync with destructive steps gated |
| **Backup** | Portable `.sql` scripts for SQLite, `pg_dump`/`mysqldump` for servers, restore in either direction, backup list |
| **Automation** | Jobs with a schedule (`03:00`, `every 15m`, manual), one command each, in-app scheduler tick, `nook-cli job serve` for cron, run log with durations |
| **Monitor** | Process list with kill, server status, server variables with filter, dashboard of the biggest tables |
| **Privileges** | Users and roles, grants, create/drop users, set passwords, grant/revoke |
| **Reports** | Aggregates (`count`, `sum`, `avg`, `min`, `max`, `count distinct`), charts (column, bar, line, area, pie, scatter, table), pivot tables |
| **Modeler** | ER diagram from the live catalog with pan/zoom, SVG export |
| **Tools** | Deterministic test-data generation, CSV/JSON import, `.sql` replay, export to CSV/JSON/SQL/Markdown, visual query builder |
| **Agents** | MCP server over stdio (every command is a tool) and a JSON control channel for the running app |

## Drive it from a shell or an agent

The desktop app is not the only door. The CLI and the MCP server resolve the same command ids:

```sh
nook-cli tables postgres://localhost/shop
nook-cli sql postgres://localhost/shop "select status, count(*) from orders group by status"
nook-cli dump postgres://localhost/shop --out backup.sql
nook-cli transfer --from sqlite:./shop.db --to postgres://localhost/stage --tables '*'
nook-cli datagen sqlite:./shop.db customers --rows 500 --seed 7
nook-cli ai sqlite:./shop.db "revenue per month for @orders"     # prompt to SQL
nook-cli ai providers                                            # endpoints and whether a key is set
printf '%s' "$OPENAI_API_KEY" | nook-cli ai key openai           # the key comes from stdin, not argv
```

A connection is a saved profile name or a URI: `sqlite:./app.db`,
`postgres://user:pass@127.0.0.1:5432/shop`, `mysql://root@127.0.0.1/shop`.
Run `nook-cli help` (or `cargo run -p nook-cli -- help`) for the full command list.

```sh
nook-cli mcp    # MCP server on stdio; every command id is a tool
```

The running app can also be driven over a JSON control channel. It listens on `127.0.0.1` and
requires the key written to `<config dir>/control-key.<port>`:

```json
{"id":1,"key":"…","method":"engine.run","params":{"command":"sql.run","params":{"conn":"local","sql":"select 1"}}}
```

See [`docs/control-protocol.md`](docs/control-protocol.md) for every method.

## Development

```sh
cargo test          # whole test suite
cargo xtask ci      # fmt, clippy (warnings denied), tests, layering check, parity freshness
cargo xtask parity  # regenerate docs/parity-checklist.md
```

`crates/engine/src/catalog.rs` lists the features a Navicat-class tool is expected to have, each with
the command id that implements it. [`docs/parity-checklist.md`](docs/parity-checklist.md) is the
generated coverage report, and a test keeps it above a floor that only rises. `cargo xtask parity`
(or `nook-cli parity` from an install) prints the same numbers, area by area, with the gaps named.

The rules the codebase is written to — no panics in non-test code, every behaviour a command,
layering enforced by `cargo xtask layers`, no invented colours — live in
[`docs/conventions.md`](docs/conventions.md). See [`CONTRIBUTING.md`](CONTRIBUTING.md) for setup and
how to send a change.

## Architecture

| Layer | Crate | Job |
|---|---|---|
| L0 | `nook-core` | errors, cell type, connection profiles, identifiers and literals per dialect |
| L1 | `nook-model` | catalog, result sets, grid edit plans, structure diffs, DDL, ER layout |
| L2 | `nook-ai` | chat client for the 7 providers, two wire formats, `@table` prompt context |
| L2 | `nook-drivers` | `Driver` trait; SQLite, PostgreSQL, MySQL; transfer, sync, dump, backup |
| L3 | `nook-engine` | sessions, command registry, feature catalog, parity, jobs, data generation, reports |
| L4 | `nook-mcp` | MCP server over stdio |
| L5 | `nook-ui` | egui front end, dialogs, control channel, `--shot` |
| apps | `nook`, `nook-cli` | desktop app, command line |

Every user-visible behaviour is a `CommandSpec` in `crates/engine/src/cmd/*.rs`; the ribbon, menus,
command palette, CLI, control channel and MCP tools all resolve the same ids. `cargo xtask layers`
enforces the layer ordering.

See [`ROADMAP.md`](ROADMAP.md) for status and [`docs/gaps.md`](docs/gaps.md) for the ranked work list.

## License

Dual-licensed under [`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE). Nook is an independent, clean-room project: no Navicat code, resources, icons, or
wording were copied. Navicat is a trademark of PremiumSoft Cybertech Ltd; PostgreSQL, MySQL, and SQLite
belong to their respective owners. Architecture conventions follow the
[WordCraft](https://github.com/storytold/wordcraft) project as a model.

See [`ATTRIBUTION.md`](ATTRIBUTION.md) for third-party notices.