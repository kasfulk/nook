# Nook roadmap

**Stage: alpha** · the core workflows are real and tested against live servers; the gaps are named.

> **Last reviewed:** 2026-10-29 · **Target:** Navicat Premium 17 (the workflow, not the code)

## Headline numbers (2026-10-29)

| Number | Value | Kind |
|---|---|---|
| Navicat feature catalog coverage | **192 / 225 (85.3%)** | measured (`cargo xtask parity` → [`docs/parity-checklist.md`](docs/parity-checklist.md)) |
| Commands | 238 registered ids | measured (`cargo xtask parity`) |
| Engines with a live driver | **3** (SQLite, PostgreSQL, MySQL/MariaDB) | measured: catalogs, queries, edits, transfer, sync and backup run against each |
| Engines catalogued but not implemented | 4 (SQL Server, Oracle, MongoDB, Redis) | named in the catalog, no driver |
| Tests | 749 | measured (`cargo test --workspace`) |
| Code | ~75,300 lines of Rust in 7 crates, 2 apps and xtask | measured (`wc -l` over every `.rs`) |
| Surfaces over the same registry | 5 (ribbon/menus, command palette, CLI, control channel, MCP) | measured |

**Why alpha:** the eight core workflows — connect, browse, query, edit, design, transfer, sync,
back up and schedule — work end to end against real SQLite, PostgreSQL and MySQL servers, and the
whole surface is reachable from a script or an agent. What is missing is breadth (more engines, more
importers, cloud and collaboration) and depth in a few dialogs (see the gaps).

## By dimension

| Dimension | Coverage | Note |
|---|---|---|
| Connections and security | 75% | SSH tunnels and SSL are in; HTTP tunnel, cloud connections and SSH-by-password are not |
| Data browsing and editing | 89% | the strongest area: paging, sort, filter, typed edits, pending SQL, transactional apply |
| Object design | 88% | tables, columns, indexes, foreign keys and views; trigger, function and sequence editors via `schema.triggerEditor`/`functionEditor`/`sequenceEditor` |
| Query work | 76% | run, format, complete, explain, history, snippets; no debugger, profiler or visual plan |
| AI query generation | 100% | 7 providers over 2 wire formats, a custom base URL each, `@table` context, keys in the keychain, the settings in the app's own SQLite store; no chat, streaming or embeddings |
| Transfer and sync | 55% | cross-engine transfer, row sync, structure diff and sync; name-based column mapping (`columnMapping`); no bi-directional sync |
| Backup and restore | 50% | SQLite scripts, `pg_dump`/`mysqldump`, restore, listing; no compression or encryption |
| Automation | 82% | jobs, schedules, run log, in-app tick and `job serve`, failure webhooks, editable chains; no remote agent |
| Monitoring | 56% | process list, status, variables, dashboard; no locks, replication or alerts |
| Privileges | 78% | users, grants, create/drop, password, grant/revoke; no audit report |
| Reports and modeler | 50% | aggregates, charts, pivot, ER diagram and SVG export; no report designer, no BI dashboard |
| Import and export | 73% | CSV, JSON, `.sql`, Excel (xlsx in/out), data generation, mapping wizard; no XML or DBF |
| Platform | 33% | native app, MCP, CLI and themes; no control-channel parity in the UI checklist, no localisation, no Windows or Linux build made here |

## Milestones

| Milestone | State | What it means |
|---|---|---|
| M1 Skeleton | done | layered workspace, error type, cell type, dialect quoting, DDL, edit plans, structure diff, ER layout |
| M2 Drivers | done | SQLite, PostgreSQL and MySQL: catalog introspection, typed results, server info, monitor, privileges, DDL, table rebuild |
| M3 Engine | done | `Session`, 238 commands, the Navicat catalog, parity measurement, jobs, data generation, reports, profiles, keychain, SSH tunnel |
| M4 Surfaces | done | desktop app, CLI, MCP server, JSON control channel |
| M5 Verification | in progress | 749 tests; live runs against SQLite, PostgreSQL and MySQL 8.4 (portable server; the Homebrew binary is ICU-broken and was left alone) |
| M6 Breadth | next | the top of [`docs/gaps.md`](docs/gaps.md): the four catalogued engines, then the remaining smaller items |

## Progress log

| Date | Change |
|---|---|
| 2026-10-29 | `platform.windows` and `platform.linux` land, closing the last two Platform catalog gaps: each reports whether the build ships that target and which host it is running on. They reuse `app.platform`'s build facts, add no engine surface, and lift coverage to 192/225 (85.3%). The `clippy.toml` MSRV now tracks `Cargo.toml` (1.95), and `decode_hex` uses `is_multiple_of` instead of a hand-written `% 2` whose comment still claimed 1.80. The roadmap, architecture note and new-build baseline were re-measured: 238 commands, 749 tests, ~75,300 lines. |
| 2026-10-14 | The `ai.*` commands get a UI. A **Generate SQL** band inside the Query tab: the prompt with an `@table` mention list read from the live catalog, the provider as a segmented control plus a combo for the rest, and the base URL and API key as numbered steps beside it. Every field runs an existing command, so the band adds no engine surface. The base URL — the thing `ai.setKey` never saved — is stored with the provider, because it is an address rather than a secret. The key goes to the keychain when the field is left or when a request is about to run, never per keystroke. Preferences ▸ AI lists every provider with its address and key state. The band measures itself and scrolls past a cap, so a short window loses the editor last. `ui.setAiBand` opens it over the control channel for scripted screenshots. 5 new tests. |
| 2026-10-13 | AI settings moved out of `profiles.json` into a SQLite file the app owns (`<config dir>/nook.db`): mode `0600`, one row enforced by `CHECK (id = 1)`, read once at start-up, and never written by a start — an install that predates the file keeps working from the old blob until the first settings change moves the value across. The API key stays in the login keychain. `app.platform` reports `ai_db_file`. 9 new tests. |
| 2026-10-13 | AI query generation. New `nook-ai` crate: 7 providers over 2 wire formats (OpenAI-compatible and Anthropic), each with its own base URL, plus `ai.generate`, `ai.providers`, `ai.setKey`, `ai.clearKey` and `ai.tables` — reachable from the CLI, MCP, the control channel and the desktop app's command palette. An `@table` mention in the prompt expands into that table's columns, types, keys and indexes. Keys live in the login keychain, never in the profile file. Verified end to end against a local server: both wire formats, a base URL with and without `/v1`, a 401 and a 429 and a timeout surfaced verbatim, a hosted provider with no key refused before any request is made, Ollama and vLLM sent no `Authorization`, the drafted SQL ran against a real SQLite database, and a mention matching nothing warned without blocking the request. Transport is the system `curl` as a child process — no new dependency — with the body on stdin and the headers, key included, in a config file the app creates `0600` and deletes. Coverage 69.0% → 70.0%; the headline numbers, the crate table and the test count were re-measured (155 was stale: 331). The ribbon entry and a prompt dialog were the UI phase, since shipped on 2026-10-14. |
| 2026-10-12 | Interface rebuilt on a real design system: one token set for colour, spacing, radius and type; an app bar with grouped menus, painted tabs, a status bar with chips and a collapsible log; a hand-painted table widget (sticky header, row gutter, striping, dirty markers, inline editor) shared by the grid, query results, monitor, users and reports; every text token verified at 4.5:1 or better in both themes. Verified by driving the app over the control channel and reviewing the rendered frames. |
| 2026-10-12 | First working alpha: 6 crates, 2 apps, 140 commands, 69.0% catalog coverage, 155 tests. Live verification against SQLite and PostgreSQL 17: catalog, queries, grid edits, structure diff and apply, `pg_dump` backup and `psql` restore, cross-engine transfer and row sync. MySQL driver written and unit-tested; the local server is broken (`icu4c@74` missing), so it is unverified against a live server. |
| 2026-10-25 | MySQL catalog now introspects every database the connection can read instead of only the profile's database; system schemas (`information_schema`, `performance_schema`, `mysql`, `sys`) stay hidden unless the profile points into one, and the profile's database still sorts first so existing tabs and bare-name table resolution keep their meaning. Column widths in the data grid are draggable (6px strip on each header cell's right edge, clamped to 76–2000px) and persist per connection + table through `store::load_ui_state`/`save_ui_state`; new engine command `grid.columnWidths` validates names against the grid's columns. Cross-driver structure diff matches tables by bare name only when that name is unambiguous across the target's schemas — a regression introduced by the multi-database catalog, found and fixed with two regression tests (`cross_driver_match_survives_a_multi_schema_target`, `ambiguous_bare_name_is_not_matched_across_schemas`). |
