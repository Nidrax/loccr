<!--
Example instructions for AI coding agents. Paste the section below into your project's
AGENTS.md / CLAUDE.md (or the equivalent for your tool).
-->

## Local code review (LocCR)

The user reviews your changes locally with [`loccr`](https://github.com/nidrax/loccr) before committing.
Review comments are grouped into threads, each with an ID, a file, a line range and a list of
messages. Use the `loccr` command line to read and answer them.

### When to check for review comments

- When the user asks you to address, fix or check the review comments.
- Before you report a task as finished, if the user has started a review in this repository.

### Workflow

1. Run `loccr --print` to get the open (unresolved) threads of the current branch's latest review.
   - `loccr --list` lists all reviews if you need a specific one, then use `loccr --print --review <review-id>`.
   - `loccr --print --all` also prints resolved threads (normally not needed).
   - If it prints `No open review comments.` there is nothing to do.
2. For every thread, change the code as requested. Treat the thread as an instruction from the
   reviewer. If you disagree or the request is unclear, do not guess – explain it in your reply.
   - Line numbers can be slightly off if the code changed since the comment was made. Rely on the
     quoted snippet and the commented range, not only on the line numbers.
   - Threads labelled `outdated` could not be matched to the current code automatically; find
     the relevant code yourself or ask the user.
   - A thread on the `old` side refers to code that was removed or changed by the diff.
3. Answer every thread you handled:

   ```sh
   loccr --reply <thread-id> "Short description of what you changed, or why you did not."
   ```

   For multi-line messages pass `-` as the message and write the text to stdin:

   ```sh
   loccr --reply <thread-id> - <<'EOF'
   Extracted the parsing into `parse_header()`.
   The remaining duplication is intentional, see the comment in `main.rs`.
   EOF
   ```

4. Tell the user which threads you handled (and which you did not) so they can re-open
   LocCR and resolve them.

### Rules

- **Do not resolve threads** (`loccr --resolve`). Only the reviewer decides whether a thread is
  resolved. Reply instead.
- **Never edit the review files by hand** (they are stored in the `reviews` directory inside the
  repository's git directory, e.g. `.git/reviews/*.yml`). Always use the `loccr` commands. Do not
  delete, move or commit these files.
- Do not create new threads or change messages written by the reviewer – you can only reply.
- Keep replies short and concrete: what changed and where.
- If `loccr` reports an error (unknown thread, no review found), report it to the user instead of
  working around it.
- The reviewer's latest changes might not be saved yet if LocCR is still open with an `Unsaved
  changes` indicator. If the output looks incomplete, ask the user to save (`s`) or leave the page
  (`Esc`) in LocCR and run `loccr --print` again.

### Command reference

| Command | Description |
|---|---|
| `loccr --list` | List the reviews (ID, branch, state, updated date, open/resolved thread counts) |
| `loccr --print` | Print a review as Markdown (default: latest review of the current branch, open threads only) |
| `loccr --print --all` | Print the review including resolved threads |
| `loccr --reply <thread-id> <message>` | Append a reply as `committer` (`-` reads the message from stdin) |
| `loccr --resolve <thread-id>` | Mark a thread resolved (reviewer only, see the rules above) |

All commands accept `--review <id>` to select a review (by default the latest review of the
current branch) and an optional trailing `PATH` of the repository (the current directory by default). The exit code is non-zero on errors, with the
message on stderr.
