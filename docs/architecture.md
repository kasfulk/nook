# Nook architecture

> **Last reviewed:** 2026-10-29 · **Target:** Navicat Premium 17 workflows

How Nook is built today. The conventions for working in the code are in
[`conventions.md`](conventions.md).

## Layers

~75,300 lines of Rust in 7 crates, 2 apps and `xtask` (counted with `wc -l` over every `.rs` under
each tree, tests included). `cargo xtask layers` enforces the ordering: a crate may only depend on
crates below it, and nothing below L5 depends on egui.

| Layer | Crate | Lines | Job |
|---|---|---|---|
| L0 | `nook-core` | 1.1k | `Error`/`Result`, `Value` (the one cell type), `ConnectionConfig` with SSH and SSL settings, dialect quoting and literals, `returns_rows` |
| L1 | `nook-model` | 3.0k | `Catalog`, `Schema`, `Table`, `Column`, `Index`, `ForeignKey`, `DbProcess`, `DbUser`, `ServerInfo`, `ResultSet`; grid edit plans; DDL for all three dialects; structure diff; ER layout |
| L2 | `nook-ai` | 1.4k | chat client for the 7 providers over two wire formats, endpoint normalisation, API keys, and the `@table` prompt context |
| L2 | `nook-drivers` | 6.3k | `Driver` trait + `sqlite`, `pg`, `mysql`; statement splitter; data transfer; row-level sync; `.sql` dump; `.xlsx` read (calamine) and write; backup and restore through the vendor tools |
| L3 | `nook-engine` | 32.9k | `Session` (connections, tabs, grid buffer, history, messages), 238 commands in `cmd/*.rs`, the Navicat feature catalog, parity measurement, jobs and the scheduler, data generation, reports, profiles, the keychain, the app's own SQLite store, the SSH tunnel |
| L4 | `nook-mcp` | 0.4k | MCP server over stdio: every command is a tool, connections open on demand |
| L5 | `nook-ui` | 28.5k | egui front end: design tokens, table widget, app shell, sidebar, grid, editor, designer, diagram, charts, monitor, jobs, dialogs, control channel, `--shot` |
| apps | `nook`, `nook-cli` | 1.6k | desktop binary and the command line |

## Data flow

1. **Connect.** `conn.open` starts the SSH tunnel if the profile asks for one (`ssh -N -L`, so agent
   keys and `~/.ssh/config` work), opens the driver, reads `ServerInfo` and then the whole catalog in
   one pass. The catalog is cached on the connection for the tree, completion, the diagram and every
   DDL generator.
2. **Act.** Every user-visible behaviour is a `CommandSpec` with an id. The ribbon, the palette, the
   keyboard, the CLI, the control channel and MCP resolve the same id through `Session::run`,
   which checks availability, runs the command and logs failures. Commands never touch the UI.
3. **Read data.** The grid asks for one page (`SELECT … LIMIT n OFFSET m`, with the tab's `ORDER BY`
   and `WHERE`), and the driver bounds the result at `max_rows`.
4. **Edit data.** A cell edit is coerced to the declared column type, turned into an `UPDATE` keyed by
   the primary key (`nook_model::plan_edits`) and pushed onto the tab's pending list. `grid.apply`
   runs the whole list inside one transaction and rolls back at the first failure.
5. **Move data.** Transfer reads pages of 2,000 rows and writes each page inside a transaction
   (`transfer::transfer`); sync compares both sides by primary key (`transfer::compare_rows`, a pure
   function) and writes only the differences; structure sync compares two catalogs
   (`nook_model::diff`) and applies the DDL, with destructive steps gated behind a flag.
6. **Back up.** SQLite is dumped in process to a portable `.sql` script (`transfer::dump_script`);
   PostgreSQL and MySQL shell out to `pg_dump`/`mysqldump` with the password in the environment, never
   on the command line. Restore mirrors both paths.
7. **Automate.** A job is a command plus a schedule, stored as JSON. The desktop app ticks every 30
   seconds and `nook-cli job serve` does the same for cron; both call `job.runDue`, which
   dispatches each due job through the registry and records the result.

## Interface

The front end is one design system, not a set of screens that each look after themselves.

- **`theme.rs` owns every visual decision** — colour, spacing, radius, type size, plus the small
  widgets the rest of the UI composes: `band`/`band_h` (a fixed-height horizontal bar),
  `card_frame`, `chip`, `ghost_button`, `primary_button`, `segmented`, `meter`, `keycap`, `divider`.
  No screen invents a colour or a pixel of padding.
- **Text contrast is a floor, not a taste:** every text token clears 4.5:1 against every surface it
  sits on in both themes (`text_dim` 6.6–7.5:1, `text_faint` 4.8–5.8:1). Disabled controls stay
  readable: they read as "not now", never as "not drawn".
- **`table.rs` is the only table.** Sticky header with sort carets and a `PK` tag, row-number gutter,
  zebra striping, hover and selection, amber dirty markers on pending cells, per-cell hit testing,
  and an inline editor (`Enter` commits, `Esc` cancels). The data grid, query results, monitor lists,
  the users list and report tables all use it, so they behave identically.
- **`band_h` exists because of a trap:** `ui.horizontal_centered` inside an unbounded panel expands to
  the panel's whole height, which starves everything below it of a clip rect and paints nothing. Any
  bar that sits in a panel allocates its rect first, then hands a bounded child Ui to its contents.
- **The app shell** is an app bar (identity, grouped menus built from the command registry groups,
  palette field), a tab strip (painted, accent underline, middle-click to close), the view's own
  toolbar, the sidebar, and a status bar of chips (connection, tab, pending edits, messages,
  coverage, control channel) with the log as a collapsible drawer behind the messages chip.
- **The AI band is a band, not a dialog** (`crates/ui/src/ai.rs`). The Query tab's toolbar opens it,
  and it sits between the toolbar and the script because the script is what it drafts into — a modal
  would cover the thing the form exists to fill. Every field is an existing command: the provider,
  the model and the base URL are `app.setSetting ai_*`, the key is `ai.setKey` (keychain), and the
  draft is `ai.generate` with `open_tab`, so the band adds no engine surface. The model is a text
  field with the provider's own list (`Provider::models`) as a picker beside it: a field because the
  vendors ship ids faster than this app can follow, and the list because typing a known one is
  busywork — `vLLM` publishes none and the field is the only way to name one, which is also the one
  case `Generate` stays disabled for (`AiForm::has_model`). It measures itself with a `Frame` plus a
  capped `ScrollArea` rather than a guessed height, so a provider that needs no key leaves no gap and
  a short window scrolls the band instead of squeezing the editor away. The scroll area does not clip
  horizontally, so every row measures itself against the pane and gives up its least-important part in
  turn — the head's subtitle, then its hint, then the footer's sentence — rather than widening the
  band and dragging the right-aligned buttons off the pane; `the_band_holds_at_the_narrowest_pane_the_app_permits`
  holds that at the 236px the shell's minimum window leaves. The base
  URL sits with the provider and not with the key, because it is an address rather than a secret:
  that is why `ai.setKey` never saved one. Preferences ▸ AI reads `ai.providers`, so the table it
  paints cannot disagree with what a request would use.

## Driver model

One trait, `Driver`, with defaults for everything a given engine cannot do — SQLite returns
`Error::Unsupported` for processes, users and database switching, and the message says so. Three
implementations:

- **SQLite** over `rusqlite` (system libsqlite3). Introspection through `sqlite_master` and the
  `table_info`/`index_list`/`index_info`/`foreign_key_list` pragmas; table rebuilds for DDL that
  SQLite cannot do in place.
- **PostgreSQL** over the simple query protocol on purpose: values come back as text, so `numeric`,
  `int8` and `timestamptz` keep full precision without a per-type decoder, and multi-statement
  scripts behave the way the editor submits them. Column names and types come from a prepared (but
  unexecuted) statement. TLS through native-tls.
- **MySQL/MariaDB** over `mysql`. Result metadata gives names and types, `information_schema` gives the
  catalog, `SHOW CREATE TABLE` gives DDL, and the MySQL-specific boolean and date shapes are mapped in
  one function.

## AI

`nook-ai` is provider-agnostic. The seven providers are one row of data each
(`crates/ai/src/provider.rs`), and there are two wire formats: the OpenAI one
(`{base}/chat/completions`, `Authorization: Bearer`) carries OpenAI, DeepSeek, Kimi, GLM, Ollama and
vLLM, and Anthropic's (`{base}/messages`, `x-api-key` plus `anthropic-version`) carries Claude. A
provider's base URL is a setting, so a gateway, a proxy or a local server is a change of address
rather than a change of code. `ai.generate` resolves every `@table` mention against the live catalog
into that table's columns, types, keys and indexes, names the active connection's dialect, and sends
one system turn plus the user's sentence.

Transport is the system `curl` as a child process: the workspace has no HTTP client and this phase
adds none. The request body arrives on `stdin` and the headers, including the key, come from a config
file written mode `0600` and removed once the child exits, so no secret reaches `argv`. The ceiling
and the measurements behind that choice are recorded on `crates/ai/src/client.rs`.

The six AI settings are stored in a row of `<config dir>/nook.db`, a SQLite file the app owns
(`crates/engine/src/store.rs`), rather than in the `settings` blob of `profiles.json`. The table is a
singleton by constraint — `CHECK (id = 1)`, so a second row is impossible rather than unlikely — and
the file is mode `0600`. A start reads it and never writes it, so opening the app cannot rewrite the
user's files: an install that predates the database still carries the old blob, which is read as the
fallback and moved across — database write first, then the key removed from `profiles.json` — the
first time a setting changes. The API key is deliberately not part of this: it stays in the login
keychain, which is one place this file cannot be read from.

**Ceiling: the store is plaintext, and the app owns one more file.** `nook.db` is not encrypted; mode
`0600` is the same treatment `profiles.json` gets, and it matters more than it did: besides the AI
configuration the file now holds saved snippets, query history and the tab snapshot — SQL text, any
line of which may contain a literal password the user typed. The keychain secret still never enters
this file. The schema is versioned through `PRAGMA user_version`; every open applies an idempotent
`CREATE TABLE IF NOT EXISTS` batch, so a file from any earlier build is brought up on the next
start. A start reads the store and creates it only when something is written. `app.platform` reports
the path (`ai_db_file`).

**Ceiling: a stored key is the `security` child's own argument.** `keychain_store` hands the secret to
`security add-generic-password -w <secret>`; while that child lives, the same user can read it from the
process table. This is the pre-existing path for SSH and database passwords too, and the door out (a
`setsid` between fork and exec) needs `unsafe` or a detach crate, both outside this phase's scope.

## Testing

- 749 `#[test]` functions across the workspace, 167 of them in the UI crate: quoting and literals,
  catalog types, DDL per dialect, structure diff, ER layout, statement splitting, edit planning, data
  generation determinism, report shaping, schedules, profiles, MCP JSON-RPC, the AI wire formats and
  prompt building, and every command against junk parameters
  (`commands_run_with_junk_parameters_never_panic`).
- Live checks that need a server are run with the CLI rather than in the unit suite, because they
  mutate databases:
  - PostgreSQL 17: `nook-cli tables|sql|describe|ddl|monitor|users`, transfer SQLite → PostgreSQL,
    `sync-data` dry run and apply, `diff-structure` and `--apply`, `dump` (pg_dump) and `restore` (psql)
    into a fresh database.
  - MySQL/MariaDB: the same battery, run against a portable `mysql-8.4.3` tarball on port 3307
    (the Homebrew `mysqld` is ICU-broken and was left alone): live-check, transfer SQLite → MySQL,
    `sync-data` dry run and apply, `diff-structure`.
- `cargo xtask ci` runs fmt, clippy with warnings denied, the workspace tests, the layering check and
  the parity freshness check.
- UI verification: `cargo run -p nook -- --shot /tmp/shot.png` renders a frame and writes a PNG.
  For a real check, start `nook --demo --control <port>` and drive it over the control channel:
  `ui.selectTab` (a negative index shows the start page), `engine.run`, `ui.setEditorText`,
  `ui.selectResult` and `ui.setResultView` (so every statement of a run and both result views can be
  shot), `ui.setTheme` (so both palettes can be shot without touching the window), `ui.openConnection`,
  `ui.setConnectionTab` and `ui.setConnectionField` (so the connection editor's own frames — a profile
  opened by id, each step, a field typed into — can be shot the same way, which is the only way a
  dialog can be photographed at all), `ui.shot` per view, `ui.quit`. The frames are then reviewed as
  images, and anything visual is fixed and re-shot. `ui.shot` writes the file and leaves the app
  running; only a `--shot` launch quits after capture.
- Dialog frames are also asserted headlessly, in `dialogs.rs`'s own tests: `egui::Context::run_ui`
  paints the app's real `show()` into an off-screen context, and the tests read the painted shapes —
  a band's height, a button's fill, a label's ink — rather than a screenshot. The context is driven
  with a clock, because an `Area` fades in over `animation_time` and a frame with no time on it is a
  frame painted half-transparent; and a frame is run three passes, because the first only lays the
  modal out and the position it derives from that placeholder is 68px low.

## Known design choices and their ceilings

- **A single statement runs in the background; a script blocks.** `dispatch` routes a one-statement
  `sql.run` to `sql.runAsync`, so the frame keeps drawing, rows land as they are read, and `Stop` has
  a running job to cancel — the editor paints that state (a `.prog` hairline, an `Executing…` pill, a
  live `Stop`) from `runner.running_for_tab`. A script of more than one statement still calls the
  blocking `sql.run`, because the runner streams one statement and pretending otherwise would drop the
  rest; there is no running state to paint and `Stop` sits disabled. Making a whole script async is
  the open work named in `docs/gaps.md` §7. The pane never paints a spinner of its own: it shows the
  run's live row count, the same number the register's note shows, because two places describing one
  run differently is the failure this avoids.
- **Cancel needs a second connection.** A server cannot be asked to cancel the statement the only
  connection is busy executing, so `Runner::cancel` dials a sibling from the same profile (password
  included, via `dial_config`) and issues `pg_cancel_backend` / `KILL QUERY`. SQLite has no such call:
  it stops at a page boundary and the command says exactly that instead of claiming a stop.
- **A server-side cancel is `Cancelled`, not `Failed`.** Postgres answers a cancelled statement with
  an error; the worker's own flag is the only thing that knows the user asked, so the flag decides
  the verdict.
- **A background run's pages land in one `StatementRun` that grows.** There is no second result type:
  `SqlState::results[..].result` gains rows, and the register paints its `row_count()` live. The
  ceiling is that the rows still accumulate in memory, bounded by `max_rows` — streaming to disk is
  phase 5's `export.stream`.
- **The plan view is offered only when the tab has a plan.** `sql.explainVisual` normalises each
  engine's own output (`FORMAT JSON` on Postgres and MySQL, the `(id, parent, detail)` rows on
  SQLite) into `nook_model::plan::Plan`; the UI parses nothing. The ceiling is a tree, not a flame
  graph: no cost heat-map across edges.
- **The register has three views, grid, JSON and plan.** `docs/edge-cases.md` and the design's mockup
  show a `Text` view too; it is not built, and `ui.setResultView` lists only the three it accepts.
- **A result tab cannot be closed.** The register's tab row is one tab per statement of the last run —
  the run's own shape — so closing one would ask what the editor should then run. Re-running a script
  reopens every tab it holds; the upgrade path is a per-tab `closed` flag the next run respects.
- **The register's staleness is the text the run's request came from.** `SqlState::ran_script` is
  stamped in `begin_run` from the text the command was issued against: the `sql` it resolved —
  `sql.run`'s parameter — which is the tab's own text when the command was given none. The three
  paths that cut a statement out of that text — `run_statement`, `sql.runSelection` and `sql.explain`
  — run their statement but are stamped with the text they cut it from, not with the cut: an
  unchanged, newline-terminated script used to report
  `stale · the script changed since this run` the moment its own statement ran, because the register
  compares the editor's whole text. The register records `ran_script` and `stale` is that comparison,
  in both directions — a rerun of on-screen text clears it, and a run whose source differed from the
  editor raises it. The rule before this compared the two splits, which was wrong in the same two
  ways. `SqlState::statements` still carries the split for the editor's own markers, but nothing
  compares it.
- **Explain is a run.** `sql.explain` goes through the same `begin_run`/`record_run` every other run
  does, so the plan lands in `SqlState::results` as one statement of its own (`EXPLAIN <stmt>`) with
  the driver's own duration and result set: the register paints it in a tab, and the stale rule above
  works on it. It used to fill `SqlState::last_result` and a `result_label`, which only the editor's
  own result table read — the reason the plan had nothing to appear in once P5 deleted that table.
  Both fields are gone.
- **A query result is editable only when the statement is one table's rows.** `grid.editQueryResult`
  accepts a single `SELECT … FROM <one table>` whose projection carries the table's primary key, and
  refuses everything else by name: `GROUP BY`, `HAVING`, `DISTINCT`, `JOIN`, `UNION`, a CTE, a window
  function, an aggregate, a subquery, two tables, an unnamed expression, or a missing key column.
  The three refusals that matter are one table, a primary key, and the key in the projection — without
  the last there is no row to update, and guessing is how a client rewrites 400 rows at once. Once
  accepted, the tab reuses the data grid's whole edit path: the same pending list, the same planner,
  the same one-transaction apply. A query-sourced grid reloads by *re-running its statement*, and the
  page-query commands (`grid.sort`, `grid.filter`, `grid.fetchPage`, `grid.setPageSize`) refuse it
  with a named error rather than storing an order they cannot apply — a grid that reports a sort it
  did not perform is worse than one that says no. `sync_grid_table` skips the table-shape comparison
  for the same reason: a narrower projection is legitimate, and reloading would let an unordered
  statement return its rows in another order under an index-based row reference.
- **Raw mode maps a statement back or refuses it.** `grid.rawMode` lists the exact `UPDATE` each
  pending edit will run and accepts an edited one only when it re-derives to exactly one staged edit:
  the shape is `UPDATE … SET col = literal WHERE …`, the `WHERE` is matched against the pending
  edits', and an ambiguous match needs `index`. Anything else is refused, because appending arbitrary
  SQL to the pending list turns a review pane into a way to run SQL without saying so. Arbitrary SQL
  belongs in the editor.
- **Safe mode is a keychain token behind a live connection test.** `editor.safeMode` mints a token
  into the keychain and hands it back only after a real `conn.test` made with the password the
  *caller* supplies — reading the stored password instead would hand the token to any script that can
  send two commands. The gate is keyed on the setting alone and on the *tab's* connection, not on
  `active_conn`: with a per-connection "token exists" test, enabling safe mode while connected to A
  would leave B ungated. The ceiling is honest and belongs in the setting's own help text: an
  attacker who can read the keychain already has the credentials, so this stops a stray script and a
  slip of the mouse, not a determined local process.
- **A bounded pill row hides pills instead of scrolling.** The register's `.rtabs` and the editor's
  `.strip` are `nowrap` flex rows in the design with the band's own fact pinned to the right, and
  nothing stopped the pills painting over that fact and past the pane: the run's total was clipped
  mid-glyph at five statements on the 776px pane and dropped altogether at six, and three ordinary
  statements put the strip's chips over its own facts. `theme::fit_row_with_fact` plans the row first
  — the fact keeps the width it measured, the pills that fit are painted, the rest are counted in
  `+N more`, and the window is built around the pill the caller names, so the register opens on the
  run's last statement. The shell's own minimum window does not leave the row the design's 776px but
  236, so three more things give way there, in the order that costs the user least: the fact (a run's
  total is the row's summary, and the tabs' own `{rows} ms` facts still say what each statement cost);
  then the `+N more` counts (the editor's toolbar counts the statements and the register's `.rbar`
  note names the ordinal `of` it); and then the volume a pill prints, which the row's own caller
  elides to the band — a register tab gives up its facts first and then its statement's word, and a
  strip chip elides its label. **Ceiling**: a pill too wide to elide into the band it is given is
  dropped whole, so a band narrower than one of them — about 60px of ordinal, ellipsis and paddings,
  against the 236 the shell's minimum window leaves the pane — paints none of the pills, and the row's
  fact is never painted in their place. **Upgrade path**: a horizontal `ScrollArea` inside the band,
  which needs a band as wide as its content — `band_fill` is the pane's width and its fill stops at the
  pane's edge.
- **The register's `Export` is painted disabled with the engine's own reason.** The register shows one
  statement's result, but `grid.export` reads the *active tab's* data grid and refuses a query tab —
  after a file picker has already asked the user for a path. So the control is off and its hover text
  is `cmd::grid::no_grid_reason()`, the same string the refusal carries, which is the rule the disabled
  `Stop` and the grid's `editable` gate follow. The upgrade path is a command that exports a result set
  the caller hands over — `tools.exportQuery` re-runs the SQL and is a different feature — at which
  point the button's action comes back.
- **The editor's toolbar is a cut-down version of the design's.** The design's full set plus its
  connection chip does not fit the pane's 776px, measured by the test at the same sizes: `Chart`,
  `Pivot`, `Save snippet` and `Export ▾` are dropped, and the connection chip with them, because the
  status bar already names the connection. The kept set is `Run`, `Run statement`, `Stop`, `Explain`,
  `Format`, `Clear` and the caret read-out. The ceiling: an overflow menu, or icons instead of labels,
  when one of the dropped actions is asked for.
- **History's surface is gone.** The editor's `History` band (`.hist`, the run list the mockup shows
  above the register) is not painted: `SqlState.results` is the current run only, and the register's
  tab row already names each statement's outcome. The ceiling is a session-level run log, which the
  engine would have to keep.
- **Text protocol for PostgreSQL.** `numeric` precision is exact and the code is small; the price is
  no binary results and no bind parameters. A prepared path can be added behind the same trait when a
  feature needs it.
- **Client-side literals for edits and transfers.** One function (`sql_literal`) escapes values for the
  grid, the import wizard, the transfer, the sync and the dump, which is why they share one code path.
  Parameterised execution is the upgrade path for very large or binary-heavy loads.
- **Generated DDL for PostgreSQL.** Postgres has no `SHOW CREATE TABLE`, so `schema.ddl` builds the
  statement from the catalog: indexes and foreign keys included, formatting not identical to
  `pg_dump`. MySQL and SQLite ask the server.
- **ASCII-safe SVG diagrams.** No external fonts, no scripts, escaping tested.
- **Chip weight.** The design gives chips a 600 weight; egui's bundled proportional family has one
  weight, so chips read by colour, size and a pill radius instead. Bundling a bold family is the
  upgrade path if the weight ever has to carry meaning.
- **Painted activity glyphs.** The rail's five icons are hand-drawn into a `Rect` from the design's
  own SVG paths (`ui::icons`), so there is no icon font and no asset bundler; the ceiling is that each
  new glyph is code, and a second icon family with different stroke rules would need a real icon
  source. `ponytail:` the sampler handles the three-segment cubics the design uses and nothing else.
- **Rail semantics.** The rail speaks the app's verbs, not the mockup's marketing copy: a report is a
  chart built from a query result, and there is no "new report" verb, so Reports selects an existing
  report tab and is disabled when none is open. Fake workspaces would have needed a state the engine
  does not have.
- **No loading skeleton in the grid.** The design draws a `.sk` state (8px `surface_3` bars inside the
  row rhythm, pager disabled, a ghost Cancel) for a page that is on its way. Every load in this app is
  synchronous — `grid.rs` reads a page, a count or a schema and returns with the result in the same
  frame — so no frame exists in which the grid has no rows *and* knows that more are coming. Painting a
  skeleton from a flag that is set and cleared inside one frame would be a state nothing can ever show.
  The upgrade path is real asynchrony (a job that reports its own progress); the design's state can be
  added then with the flag it needs.
- **The workbench pane is flush and each surface carries its own insets.** `app::show` gives the
  central pane a frame with no margin — the design's `.reg-grid` has none either — so a surface that
  needs air inside the pane states it itself (`views::welcome`, the query editor and the monitor
  dashboard have no such rule in the design, so they sit flush and were re-measured rather than
  padded). Inside the grid, `show` zeroes `item_spacing.y` so its bands stack hairline to hairline.
- **The grid asks the widget for its head alone on an empty page.** `TableView::height = 0` is the
  sentinel: `table::show` paints the head inside a horizontal scroll area and returns, and the caller
  owns the space under it (the `.empty` block's own `padding:28px`). The head keeps that scroll area
  because a table wider than the pane would otherwise be clipped with no bar to reach the columns past
  the edge. Its height is the head's own, so the block still lands one head below the hairline — the
  scroll area collapses to its content, and the horizontal bar is charged to the width, not the height,
  so an overflowing head does not push the block down either. The viewport's floor is `HEAD_H` rather
  than egui's `min_scrolled_size` default (64px), which is what put the block 72px down before; the
  floor is idle while egui's bars float, and holds the band if a future style stops them floating.
  `ponytail:` a zero-height body is not expressible
  — nothing asks for one — so the upgrade path is a `Body` enum (`HeadOnly | Scroll(f32)`) if a caller
  ever wants a body of no height.
- **The grid's badge and notice marks have one definition each.** The head reserves
  `theme::pk_badge_width` and paints through `theme::paint_pk_badge`, so the toolbar's `.pkbadge` and
  the head's cannot drift apart; `theme::notice_band` takes the band's leading `icons::Glyph` and its
  message `FontId`, which is how the `.errband` gets `.ei` plus a `--t-sm` mono message and the
  `.robanner` gets `.ri` plus its proportional `--t-xs` paragraph.
- **One `editable` flag decides what the grid offers.** `GridState::editable` is false when the engine
  would refuse an edit (`grid.editCell`, `grid.setNull` and the rest go through `pk_values`), and every
  surface that names such an action reads it: the toolbar's `+ Row` / `Duplicate` / `Delete`, the
  selected-cell band's `Double-click` keycap and `Set NULL`, and the widget's own inline editor and
  dirty stripe. A surface that forgot it would advertise an action the engine rejects.
- **The grid prints only the facts it holds.** `.matches` shows the count `grid.countRows` returned —
  and only until the grid starts asking for different rows: `grid.filter`, `grid.clearFilter`,
  `grid.sort` and `grid.apply` drop it, because a count read under the previous request is not this
  page's match total (an applied edit inserts, updates or deletes rows of the set just counted).
  With no count there is nothing honest to print but the page in hand (`50 rows on this page`). The
  `.robanner`'s reason comes from the binding and the schema — not from the page — in the order
  `GridState::editable` checks them, so a keyless table is never told it has no primary key because a
  page came back empty.
- **Bar bands.** `theme::bar_frame` contributes no stroke and no margin, and every panel is built with
  `show_separator_line(false)`, so a bar's height is its token plus the hairline painted inside it.
  The ceiling is that any future panel must paint its own edges to be visible.
- **Density reaches the data grid and the results, not the catalog lists.** `theme::row_h` turns the
  persisted `density` preference into 26px compact or 30px comfortable rows, and only the two call
  sites that render a real grid decide per frame (`grid.rs`, `editor.rs`); `views::list` pins
  `theme::ROW_H`. The design demonstrates comfortable rows on its grid register, so the lists keep the
  compact measure until an explore register asks otherwise — then `list` takes a row height too.
- **The explore preview is a page, not a pager.** `nook_engine::cmd::preview_table` runs the same
  `page_select` the grid does, without a history entry and without registering a command of its own, so
  `cargo xtask parity` stays honest. `ponytail:` there is no control-protocol surface for it and no
  second page; add a command and an offset argument when the preview needs to scroll past
  `PREVIEW_ROWS`.
- **The inspector's Foreign-keys tab states the outgoing half only.** Listing what references a table
  needs a scan no command performs, so the model paints the `Outgoing` group and, when it is empty,
  the design's sentence rather than a second group it cannot fill. `ponytail:` add a
  `schema.scanReferences` command and a second `Group` when the incoming half is worth a round trip.
- **The `.fchips` add chip has a solid border.** The design draws `border-style: dashed`; egui has one
  stroke per shape and no dash pattern, so the chip keeps the design's weight, radius and `border`
  colour with a solid hairline. `ponytail:` the ceiling is that the chip is not visually distinct from
  a real filter chip at a glance; the upgrade path is painting the dashes by hand along its top and
  bottom edges.
- **The `.stalestrip` carries the last order and filter, not the last rows.** `load_grid` clears
  `GridState::result` when the page fails, so the strip can say what the grid is still asking for but
  the design's `.stale` block — the previous page kept on screen under the error — has no rows to
  paint. `ponytail:` the upgrade path is keeping the previous `ResultSet` in `GridState` and painting
  it inside a `.stale` wrapper.
- **One changes surface, two thirds of the design's.** The design draws both the toolbar's `.pend`
  cluster and a drawer with a `.dfoot`; this app keeps the persistent `.pend` cluster (only while the
  buffer is non-empty, as the design's own `0 pending` states do) and the drawer, whose footer now
  carries `Copy to SQL`, Revert and `Apply n changes`. `Copy to SQL` writes the statements one per
  line, because each `edit.sql` already ends in `;` and a `;` separator would leave an empty statement
  between every pair. `ponytail:` the drawer body is still the
  `CollapsingHeader` of `kind · sql` lines rather than the design's grouped `.dgrp`/`.dchg` list; the
  grouped list needs per-kind headers and a per-change undo, which the engine's `Vec<PendingEdit>` can
  address when the drawer is worth the second pass.
- **The SSH password lives in the login keychain and nowhere else.** `conn.sshPassword` stores it under
  the connection's own account — `profiles::ssh_account(id)` is `{id}#ssh`, so it cannot collide with
  the database password the `{id}` account holds — and `conn.delete` removes both accounts. The tunnel
  hands it to `ssh` through an askpass helper: `SSH_ASKPASS` is the app's own binary with
  `SSH_ASKPASS_REQUIRE=force`, `--ssh-askpass` prints one line from the keychain, and the connection id
  travels in `NOOK_ASKPASS_CONN`, because ssh's askpass protocol passes only the prompt and no
  arguments of its own. The secret is therefore never in an argument, an environment variable or a
  file, and `ConnectionConfig` gained no field, so the SSH password never reaches `profiles.json`.
  Where there is no keychain — every platform but macOS — there is no source for the helper either:
  `conn.sshPassword` refuses with `profiles::NO_KEYCHAIN`, `SshTunnel::ssh_args` drops `BatchMode=yes`
  so `ssh` may ask at all, and the tunnel inherits the app's stdin for that prompt. `SshSecret` names
  the three states, and `SshSecret::for_connection` answers `None` where there is neither a stored
  secret nor a terminal to ask on, which is what keeps a protocol stream (MCP's stdin) out of ssh's
  hands. **Ceiling**: the fallback prompt lands on the terminal the app was started from, so a launch
  with no terminal has no way to answer it and fails with ssh's own sentence, and the helper exists on
  macOS only — the same ceiling as the keychain it reads. **Upgrade path**: an in-app prompt whose
  answer is written to the keychain for the length of the run, which is this plumbing with the
  persistence swapped for a session (`docs/gaps.md` §12).
- **The connection editor and its confirmation are modals; the other five surfaces are still
  windows.** `dialog_shell` is `egui::Modal::new(id).backdrop_color(tokens.scrim).frame(..)`, so the
  scrim, the centred position, the radius-6 `stroke_strong` frame and the click-outside-to-close
  behaviour come from one place, and the editor (560 wide) and the confirmation (440) are its two
  users. The command runner, Generate data, Export, Preferences and the `Cmd+K` palette still paint
  through `egui::Window` with the app's own frame: placed rather than centred, dimming nothing, and
  leaving the panes behind them live. **Ceiling**: two chromes — a dialog's scrim, radius and border
  are the design's only where `dialog_shell` is used, and the five do not block input to what is under
  them. **Upgrade path**: the next slice swaps each `egui::Window::new` for `dialog_shell` with its
  own width; the actions inside do not change, which is the reason the two can live apart at all.
- **The TLS step ships the mode, not the certificate fields.** The design's step draws three
  certificate paths; `ConnectionConfig` has no field for them and no driver reads one — `native-tls`
  verifies against the OS trust store — so the step paints the five modes `SslMode::parse` accepts and
  says what is true in the design's own `.helper` place (`TLS_HELPER`): the certificate paths have no
  field in this version. **Ceiling**: `verify-ca` and `verify-full` can only trust a private CA that
  the OS already trusts, so a self-signed chain needs a system-level install. **Upgrade path**: three
  fields on the TLS config, which is where `profiles.json`'s shape changes, plus the connector's
  certificate source.
- **A profile's colour is carried and never painted.** The editor's form holds the profile's `color`
  and a save writes it back (`ConnectionConfig.color`), and nothing in the app reads it: the design's
  colour picker — the fourth control of the General step's own row — is not shipped, and the tree
  paints no per-connection dot. **Ceiling**: a colour can be set in `profiles.json` by hand and has no
  effect on any frame. **Upgrade path**: paint it where the design does — the picker here and the
  connection's chip in the tree — which is one helper and one call site.
- **The Advanced tab is this app's own, and holds the destructive action.** The design's fourth tab is
  never drawn open in the mockup, so there is no design for its body; what it holds is the profile's
  own facts (engine, connected or not, group), the sentence that deleting removes the profile and the
  passwords saved with it and leaves the server alone, and `Delete connection` — the one action the
  design gives no home. It is a `danger` button that opens the confirmation, which is where the
  profile's name is typed, and `conn.delete` refuses without `confirm: true` in the same breath.
  **Ceiling**: the tab's content is unreviewed by the design, and the delete sits one click deeper
  than a design that had drawn it. **Upgrade path**: when the design draws the tab (pooling, session
  settings), the action moves to wherever that names.
- **The AI band's fields write as they change.** Choosing a provider, or typing in the base-URL
  field, runs `app.setSetting` at the end of the frame it changed in — one store write plus one
  `profiles.json` rewrite per keystroke in the address field, and `dirty` coalesces the fields so it
  is never more than one write per frame. **Ceiling**: on a slow disk a long address stutters. At
  ~2 KB the write is under a millisecond, which is why it is not debounced today. **Upgrade path**:
  flush on focus loss, which needs the value compared against the session's own before the write so a
  click straight from the field to Generate cannot lose it.
- **The AI band's `Generate` blocks the frame.** `ai.generate` shells out to `curl` and waits for the
  model, on the UI thread — the same ceiling `sql.run` has, recorded under §7 of `docs/gaps.md`. The
  band closes when the draft lands, so the wait is visible as a still window rather than a spinner.
  **Upgrade path**: the job path the same gap note names, which would hand the band a job id to poll.
- **A table rebuild takes a diff, never a column list.** `schema.tableRebuild` reads the live columns
  from the catalog and applies `rename` / `retype` / `nullable` / `default` / `drop` / `add` to a
  clone; the driver then rebuilds from the result. A command that took "the new columns" would invite
  every caller to reconstruct that list by hand, and the first one that forgot a column would silently
  drop its data. Columns are paired to their source by **ordinal**, not by name, because a rename keeps
  its position and a name pair would lose the data it renamed. Three checks run before the commit: a
  rebuild inside an open transaction refuses (the pragma cannot change inside one), every foreign key
  pointing at the table must still resolve to a column it has, and `PRAGMA foreign_key_check` scoped to
  the table must return nothing — a failure rolls back and names the offending rows. **Ceiling**: the
  rebuild is SQLite-only; Postgres and MySQL alter in place and `schema.tableRebuild` refuses there.
- **A forward-engineered cycle needs a server that can defer a key.** Tables are emitted in dependency
  order, and a key that cannot be ordered is lifted out of its `CREATE` and emitted as
  `ALTER TABLE … ADD CONSTRAINT` afterwards. SQLite has no such statement, so on SQLite a cycle is
  refused with a sentence naming it rather than emitting a script that dies halfway. **Upgrade path**:
  emit the cycle as two `CREATE`s plus a table rebuild per side, which is the only arrangement SQLite
  accepts.
- **Partitions are read on every engine, written for range-by-date only.** The bounds come from the
  server's own catalog (`pg_get_expr(relpartbound)` / `information_schema.PARTITIONS`), never from
  re-parsing a DDL string. `schema.createPartition` validates the bounds as real dates before any SQL
  runs, so `2026-13-01` is refused in the dialog rather than by the server after it closed. **Ceiling**:
  no list, hash, default partition, or MySQL `REORGANIZE`.
- **The diagram is a document, and PNG is not part of it.** `diagram.reverseEngineer` returns the
  catalog with box positions and the tables themselves, so a saved model reads without a live
  connection; positions are stored per `conn + schema` in `ui_state`, which is why a drag survives a
  reopen. `diagram.exportPng` is **dropped**: the only `ColorImage` in the tree is a screenshot of the
  running window, and rendering a diagram offscreen would need an egui painter into an offscreen
  buffer — a rasteriser that does not exist. SVG stays the commitment. **Upgrade path**: one
  offscreen `egui::Painter` into a `ColorImage`, then the `png` encoder already in the tree.
