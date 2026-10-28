# Nook — Agent Guide

This file is the short pointer for coding agents. Read [`docs/conventions.md`](docs/conventions.md) before writing code — it is the authoritative rulebook for this repository.

## Commands

- `cargo test` — run the test suite.
- `cargo xtask ci` — the full gate: `fmt --check`, `clippy -D warnings`, tests, `cargo xtask layers`, `cargo xtask parity-check`.
- `cargo xtask parity` — regenerate [`docs/parity-checklist.md`](docs/parity-checklist.md) from the feature catalog.
- `cargo xtask layers` — enforce the crate layering (L0..L5).

## Do not touch

- Command ids in [`crates/engine/src/catalog.rs`](crates/engine/src/catalog.rs). They are the contract between the GUI, CLI, control channel, and MCP surface. Renaming one breaks every surface.
- [`docs/parity-checklist.md`](docs/parity-checklist.md) by hand. It is generated; run `cargo xtask parity` instead.
- `unwrap`, `expect`, or `panic!` in non-test code. See [`docs/conventions.md`](docs/conventions.md) for the no-panic rule and the error-handling conventions.
- The confirm gate on destructive engine commands. Dropping a table, truncating, or overwriting data must keep its confirmation path.

## Tool directories

- `.serena/` is committed project configuration (it has its own `.gitignore`).
- `.claude/`, `.agents/`, `.agile/`, `graphify-out/`, `opendesign/` are local-only and gitignored. Do not commit them.

## Layer ordering

Enforced by `cargo xtask layers`. Dependencies go downward only.

| Layer | Crate |
|---|---|
| L0 | `nook-core` |
| L1 | `nook-model` |
| L2 | `nook-ai`, `nook-drivers` |
| L3 | `nook-engine` |
| L4 | `nook-mcp` |
| L5 | `nook-ui` |
| apps | `nook`, `nook-cli` |

See [`README.md`](README.md) for the full architecture table and [`ROADMAP.md`](ROADMAP.md) for status.
