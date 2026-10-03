# CLAUDE.md

LocCR (Local Commit Review) is a Rust TUI (ratatui) for reviewing local changes – typically the ones an AI agent just made – before committing and pushing. Comments are threaded, saved as YAML inside the repository's git directory, and can be read and answered by an AI agent through a non-interactive CLI.

The source of truth for the design is GitHub issue #1 ("Tech spike and implementation plan"): it contains the full spec (review file schema, position semantics, key bindings, startup rules, stale-review recovery, limits) and the work plan. The milestones M1–M19 are its sub-issues (#2–#20); each lists deliverables, tests and acceptance criteria. Read the relevant issue before implementing a milestone, and do not re-litigate decisions recorded there. `AGENTS_EXAMPLE.md` documents the CLI contract for AI agents and must be kept in sync with the CLI.

## Commands

```sh
cargo build
cargo test                                  # needs the `git` CLI installed
cargo test --test <file> <name>             # a single integration test / test name filter
cargo clippy --all-targets -- -D warnings   # must be clean
cargo fmt --check
cargo deny check licenses                   # GPL-compatible dependency licenses
cargo run --example keydump                 # manual: dump key / mouse events of the terminal
```

## Architecture (planned, see issue #1)

Single package, **library + thin binary**: `src/lib.rs` holds everything, `src/main.rs` only maps args and errors to exit codes.

- `text/` – lines and line endings, positions, ranges, anchors, word motion
- `model/` – review model, YAML, store, three-way merge
- `git/` – repo discovery, base resolution, commits, change set (`gix`)
- `diff/` – line diff, blocks and context, large-file classes
- `recovery.rs`, `session.rs` – stale-review recovery; `ReviewSession` shared by the CLI and the TUI
- `cli/` – `--list`, `--print`, `--reply`, `--resolve`
- `tui/` – pure `update(AppEvent) -> Vec<Effect>`, pages, editor, palette, polling

Core rules that are easy to get wrong:

- Positions are **1-indexed**, `column` counts Unicode scalar values, `end` is **exclusive**, a tab is 1 column, line terminators are never part of a line.
- Reviews live in `<common git dir>/reviews/` (`gix` `common_dir()`), never a hardcoded `.git/reviews`; the filename is the review ID, not a commit hash.
- Saving is always: re-read from disk, three-way merge, atomic write (temp file + rename). The AI writes to the same files via `--reply`, so never overwrite blindly.
- Deletion marks are in-memory only and are never written to the review file.
- Agents (and this codebase's tests) never edit review YAML by hand; they use the CLI.
- The diff of the base against the working tree is built with `gix` (virtual in-memory tree + rewrite tracking). Known quirk: a copy source loses its own `Modification` entry, re-add it.

## Conventions

- Rust edition 2024, MSRV from `Cargo.toml` (`rust-version`); Linux and macOS only (POSIX paths).
- Keep the TUI logic testable: state transitions in pure functions, snapshot tests with ratatui's `TestBackend` (`insta`), integration tests against real temporary git repos built with the `git` CLI (see `tests/common/mod.rs`, hermetic environment, fixed dates).
- Everything must be reachable from the keyboard; mouse is optional. Filter key events to `Press` / `Repeat`.
- Licensed GPL-3.0-or-later: keep dependency licenses compatible (`cargo deny`).

## Git workflow

- Main branch: `trunk`.
- Branches that address an issue are named `<issue-id>-description`, e.g. `2-text-and-positions-core`.
- Reference the issue in commit messages / PRs (`Part of #1`, `Closes #2`).
- AI attribution: end commit messages with an `Assisted-by:` line (e.g. `Assisted-by: Claude Sonnet 5.5`). Do **not** use `Co-Authored-By` for the AI.
- Only commit or push when asked.
