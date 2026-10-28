# Contributing

Thanks for your interest in Nook. This document covers setup, the rules this codebase follows, and how to send a change.

## Setup

Clone the repository and install Rust via [rustup](https://rustup.rs). The repository pins its toolchain in `rust-toolchain.toml`; rustup reads that file and installs the right version automatically.

```sh
git clone https://github.com/kasfulk/nook.git
cd nook
cargo test              # whole test suite
cargo xtask ci          # fmt, clippy (warnings denied), tests, layering check, parity freshness
```

`cargo xtask ci` is the gate that runs in CI and must pass before a pull request merges.

## Read the conventions first

[`docs/conventions.md`](docs/conventions.md) is the rulebook. The important parts:

- **No panics in non-test code.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Production crate roots deny them via clippy.
- **Errors are `Result<T>`** through `nook_core::Error` and `?`. A feature that cannot work says so with `Error::Unsupported` and a sentence the user can act on.
- **Input-derived values are hostile.** SQL text, cell values, file contents, imports, command parameters, MCP arguments: bound row counts, quote every identifier, escape every literal, clamp sizes, validate identifiers that cannot be parameterised.
- **Destructive operations ask first.** `schema.dropTable`, `schema.dropColumn`, `schema.truncateTable`, `schema.dropIndex`, `schema.dropView`, `schema.emptyDatabase`, `monitor.kill`, `priv.dropUser` require `"confirm": true` unless the `confirm_destructive` preference is off.
- **Everything is a command.** Every user-visible behaviour is a `CommandSpec` in `crates/engine/src/cmd/*.rs` with an id, a label, a group, documented parameters, and a test. The ribbon, menus, palette, keyboard shortcuts, CLI, control channel, and MCP all dispatch by id.
- **Feature ids are catalogued.** `crates/engine/src/catalog.rs` lists the features a Navicat-class tool is expected to have, each with the command id that implements it. Implementing a catalogued feature means adding a command with that exact id — never rename one to move the number.
- **Parity is measured.** `cargo xtask parity` regenerates [`docs/parity-checklist.md`](docs/parity-checklist.md); a test keeps coverage above a floor that only rises.

## Your first change

Pick something from [`docs/gaps.md`](docs/gaps.md) — the ranked work list, open entries. Build the release binaries:

```sh
cargo build --release -p nook -p nook-cli
# or
./install.sh            # builds and copies to ~/.local/bin
```

Run the desktop app on the sample database to see the current state:

```sh
nook --demo             # sample SQLite database in the temp directory
```

Add the command or test, then run the gate:

```sh
cargo xtask ci
```

If you added or changed a catalogued feature, regenerate the parity checklist:

```sh
cargo xtask parity
```

## Commit and pull request

- **Subject lines** follow the repo's history: `feat(editor): …`, `fix(ci): …`, `docs(readme): …`, `refactor(engine): …`.
- **A fix lands with a test.** The sweep in `crates/engine/src/lib.rs::commands_run_with_junk_parameters_never_panic` runs every command with junk parameters; add yours to it if it is a new command.
- **Tick the pull request template honestly.** If `cargo xtask ci` does not pass, do not open the pull request.
- **One commit per arc of work** that builds. The conventions say "commit after each arc of work that builds"; a pull request can carry several.

## Good first issues

The [`good first issue`](https://github.com/kasfulk/nook/labels/good%20first%20issue) label on GitHub marks small, well-scoped changes suitable for a first contribution.

For the bigger picture, see [`ROADMAP.md`](ROADMAP.md) (status and direction) and [`docs/gaps.md`](docs/gaps.md) (the ranked work list).

## Questions

Open an issue on GitHub or start a discussion. For security vulnerabilities, see [`SECURITY.md`](SECURITY.md).
