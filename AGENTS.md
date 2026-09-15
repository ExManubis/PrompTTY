# AGENTS.md

PrompTTY is a local-first terminal and agentic development environment forked from
[Warp](https://www.warp.dev). Rust, Cargo workspace of 70+ crates, macOS and Linux hosts only.

## Commands

| Task | Command |
| --- | --- |
| Run GUI app | `./script/run` (or `cargo run`) |
| Run headless TUI | `./script/run-tui` |
| Core tests (what CI runs) | `./script/run-core-tests` |
| Single crate tests | `cargo nextest run -p <crate>` |
| Full presubmit (fmt, clippy, tests) | `./script/presubmit` |
| Format | `./script/format` |
| First-time setup | `./script/bootstrap` (`--skip-gcloud-auth` to skip the gcloud check) |

Presubmit runs clippy with `-D warnings` on several package subsets (not `--all-features`); read
`./script/presubmit` rather than guessing the flags.

## Architecture

Two front-ends share one core:

- **Shared core**: `crates/warp_core`, `crates/warpui`, `crates/warpui_core`. A global `App` owns
  all views and models as entities; views reference each other via `ViewHandle<T>`, never direct
  ownership. `AppContext`/`ViewContext`/`ModelContext` give temporary access during render/events.
- **GUI** (`app/`): WarpUI pixel/GPU framework, Flutter-style `Element`/`View` layout, WGSL
  shaders, mouse input.
- **TUI** (`crates/warp_tui` + `crates/warpui_core/src/elements/tui`, behind the `tui` feature):
  cell-grid `TuiElement` trait painting into a `TuiBuffer`, crossterm input → `TuiEvent`. No GPU.

Feature surfaces the TUI reuses live in `app/` (`terminal/`, `ai/`, `workspace/`).

**Skills**: `gui-*` skills are GUI-only, `tui-*` skills are TUI-only, anything else applies to
both. Load the matching one before UI work and ignore the other front-end's.

## Legacy code, do not extend

- Warp cloud services (Drive, teams, billing, Oz, telemetry, autoupdate, sync) are being removed.
  Treat `app/src/drive`, `app/src/autoupdate`, and sync/telemetry plumbing as on the way out.
- The `FeatureFlag` system (`crates/warp_features`, `app/src/features.rs`) is deprecated. Don't add
  flags; write ungated implementations.

## Gotchas

- **IMPORTANT: `TerminalModel` locking.** Nested `model.lock()` calls from different call sites
  deadlock and freeze the UI. Before adding a lock, confirm nothing up the call stack already holds
  it. Prefer passing an already-locked reference down; keep any new lock scope minimal and call
  nothing that might lock again.
- `MouseStateHandle` must be created once at construction and cloned where needed. An inline
  `MouseStateHandle::default()` during render silently disables all mouse interaction. Applies to
  TUI hover/click elements too (`TuiHoverable`, `tui_collapsible`).
- Avoid `_` wildcards in `match`; exhaustive matching surfaces new enum variants at compile time.

## Code style

Rules that differ from rustfmt/clippy defaults or that clippy won't catch:

- Context parameters (`AppContext`, `ViewContext`, `ModelContext`) are named `ctx` and go last,
  unless the function takes a closure, which goes last instead.
- Remove unused parameters entirely; don't rename them to `_foo`.
- Prefer imports at the top of the file over long path qualifiers. Inside `cfg`-gated blocks a
  scoped import or a one-off absolute path is fine.
- Skip type annotations that inference handles, especially in closure params.
- Use inline format args (`format!("{message}")`).
- Never pass `Itertools::format` output to logging macros (`log::*`, `safe_*`); they may format
  twice and the formatter is single-use. Pass a `String` (`iter.join(", ")`) instead.
- Reflow comments to the formatter's 100-column width.

### Comments

- Explain *why*, never *what* or *how*. Assume a senior engineer is reading.
- Doc comments on a container (struct, enum, trait) describe the whole; member docs describe the
  members. Don't repeat one in the other, and don't repeat a doc comment at call sites.
- Don't list a function's callers in its doc comment.
- No edit-narration comments ("this used to…"); that belongs in the PR.
- Don't touch existing comments unless the code they describe changed.

## Testing

- Unit tests go in a sibling file (`foo_tests.rs` or `mod_test.rs`) included with
  `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;`. Inline `mod tests {}` fails presubmit.
- TUI screens are tested with render-to-lines tests (see the `tui-testing` skill). The old GUI
  integration harness is gone; prefer unit tests.
- After a change, verify with the narrowest check that proves it: `cargo check -p <crate>`, then
  the affected test file. Don't run the full suite or full clippy unprompted; that's presubmit's job.

## Pull requests

- Run `./script/format` and the clippy commands from `./script/presubmit` before opening a PR or
  pushing to a PR branch. They must pass; fix everything first.
- Use the template at `.github/pull_request_template.md`.
- Never open a public PR or issue disclosing a non-public security vulnerability. Point to
  `SECURITY.md` instead.
