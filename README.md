# LocCR

**LocCR** (Local Commit Review) is a terminal UI for reviewing changes locally before you commit
and push them – ideally the changes an AI coding agent just made for you.

You read the diff, leave review comments on any line (added, removed or context), and the comments
are saved next to your repository. Your AI agent can then read the open comments, fix the code and
reply to every thread, all from the command line, and you resolve the threads once you are happy.

> **Status:** early development. The design is settled, the implementation is just starting, so
> most of what is described below is not usable yet.

## Features (planned)

- Review **uncommitted changes and unpushed commits** of the current branch against its upstream
  (or `origin/HEAD`, or a local `main` / `master` / `trunk`).
- Side-by-side and unified diff with line numbers, syntax highlighting and expandable context.
- Threaded comments on a selection of text: messages from the *reviewer* and the *committer*, with a
  `resolved` state. Overlapping and nested comments are highlighted.
- Per-file *reviewed* flag, like GitHub's *Viewed*.
- Reviews survive amends, rebases and resets: LocCR detects that the code changed, tells you, and
  tries to re-locate your comments (what cannot be matched is marked *outdated*, never deleted).
- A non-interactive command line (`--list`, `--print`, `--reply`, `--resolve`) so that an AI agent can
  read and answer the review without touching the review files.
- Keyboard-first; the mouse works as a convenience where the terminal reports it.
- Works with git worktrees; reviews are stored inside the repository's git directory and are never
  tracked by git.

## Installation

LocCR is distributed as source for now. It requires Rust 1.89 or newer (edition 2024):

```sh
cargo install --git https://github.com/Nidrax/loccr
```

Supported platforms are Linux and macOS (Windows users: use WSL). A terminal with truecolor and the
[kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) gives the best
experience, but everything is reachable with plain keys in other terminals as well.

## Usage

```
loccr [OPTIONS] [PATH]
```

Without `PATH` the current directory is used. The repository is discovered from `PATH` upwards.

| Option | Description |
|---|---|
| *(none)* | Open the interactive TUI |
| `-h`, `--help` | Show the help |
| `-v`, `--version` | Show the version |
| `--list` | List the reviews (ID, branch, state, updated date, open / resolved thread counts) |
| `--print` | Print a review as Markdown (open threads only, `--all` includes resolved ones) |
| `--reply THREAD_ID MESSAGE` | Add a `committer` reply to a thread (`-` reads the message from stdin) |
| `--resolve THREAD_ID` | Mark a thread resolved (`--unresolve` reopens it) |
| `--review ID` | Select the review for `--print`, `--reply` and `--resolve` (default: the latest review of the current branch) |

### Interactive mode

The TUI has four pages:

- **Home** – the list of reviews and a `Start a new review` entry. `x` / `d` marks a review for
  deletion, `s` saves.
- **Review** – the commits and the files of the change. `Space` marks a file as reviewed, `Tab`
  switches between the groups.
- **Commit details** – hash, refs, author, date and message of a commit.
- **File commenting** – the diff of one file. Arrows move the cursor, `Shift` selects, `Ctrl` (or `Alt`)
  jumps by words, `c` adds or opens a comment thread, `r` resolves it, `x` / `d` marks it for
  deletion, `+` / `-` change the amount of context, `Tab` switches between split and unified layout,
  `[` / `]` switch to the previous / next file.

`Esc` goes back one page and saves; destructive actions (deleting a review or a thread) always need a
confirmation. The status bar shows when there are unsaved changes – save or leave the page before you
prompt your AI agent, because the agent reads the saved review.

The complete list of key bindings will be documented here once the interface is implemented.

## Using LocCR with an AI agent

1. Review your changes and leave comments in `loccr`, then save (`s`) or leave the page (`Esc`).
2. Tell the agent to address the review. Paste the snippet from [`AGENTS_EXAMPLE.md`](AGENTS_EXAMPLE.md)
   into your project's `AGENTS.md` / `CLAUDE.md` once, and the agent will know how to use
   `loccr --print` and `loccr --reply`.
3. Open `loccr` again, read the replies, and resolve the threads that are done.

## Review files

Reviews are YAML files, one per review, named `<review-id>.yml`, stored in the `reviews` directory of
the repository's common git directory (e.g. `.git/reviews/`). Agents should use the command line
instead of editing them by hand. The format (threads, messages, positions, anchors) will be documented before the first release.

## Development

```sh
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo deny check licenses
```

The tests build real temporary git repositories with the `git` command line, so `git` must be
installed.

## Documentation

- [`AGENTS_EXAMPLE.md`](AGENTS_EXAMPLE.md) – ready-to-paste instructions for AI agents.

## License

LocCR is licensed under the [GNU General Public License v3.0 or later](LICENSE).
