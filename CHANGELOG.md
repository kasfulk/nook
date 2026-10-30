# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- `platform.windows` and `platform.linux` commands report whether the build ships that target and
  which host it is running on, closing the last two Platform catalog gaps.
- Release builds now publish `checksums.txt` plus a rendered Homebrew formula (`nook.rb`) and
  Scoop manifest (`nook.json`) as release assets; see `packaging/README.md` for tap/bucket setup.

### Fixed
- `clippy.toml` now declares the workspace MSRV (1.95) instead of a stale 1.80, so lints are
  evaluated against the toolchain the project actually builds with.

## [0.1.0] - 2026-10-28

First public release. Nook is a Navicat-style database workbench for SQLite, PostgreSQL, and MySQL, shipped as a desktop app (`nook`), a CLI (`nook-cli`), and an MCP server for agents.

- Three engine drivers (SQLite, PostgreSQL, MySQL) with SSH tunnels, SSL modes, OS keychain password storage, and read-only profiles.
- Catalog browser, data grid with inline edits and pending-changes apply, multi-statement SQL editor with completion and `EXPLAIN`/`EXPLAIN ANALYZE`.
- Design tools (tables, columns, indexes, foreign keys, views), ER diagram with SVG export, visual explain and query profiler.
- Transfer across engines, row-level sync by primary key, structure comparison with gated destructive DDL, backup/restore via `.sql`/`pg_dump`/`mysqldump`.
- Jobs with schedules and chaining, `nook-cli job serve` for cron, in-app scheduler, run log.
- MCP server over stdio (every command is a tool) and a JSON control channel for automation.
- AI chat across seven providers (OpenAI, Anthropic, DeepSeek, Kimi, GLM, Ollama, vLLM) with `@table` expansion and keychain-stored API keys.
- CLI with the full command surface: `tables`, `sql`, `dump`, `transfer`, `sync`, `backup`, `restore`, `job`, `import` (CSV/JSON/Excel), `export` (CSV/JSON/SQL/Markdown).

[Unreleased]: https://github.com/kasfulk/nook/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kasfulk/nook/compare/f55b863e09e9a74a3964e18c465d83a5222b4e8d...v0.1.0
