## Description

<!-- What does this change do? One paragraph. -->

## Motivation

<!-- Why? Link to the issue this addresses, or explain the gap. -->

## How it was tested

<!-- Commands run, fixtures added, screenshots for UI changes. -->

## Checklist

- [ ] Tests added or updated. A fix lands with a test.
- [ ] `cargo xtask ci` passes (fmt, clippy with warnings denied, tests, layering, parity freshness).
- [ ] `docs/parity-checklist.md` regenerated if a catalogued feature changed (`cargo xtask parity`).
- [ ] No new `unwrap()` / `expect()` / `panic!` / `unreachable!` / `todo!` / `unimplemented!` outside tests.
