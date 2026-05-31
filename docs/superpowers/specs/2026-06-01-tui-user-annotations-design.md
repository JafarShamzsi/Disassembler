# Design: TUI User Annotations + Annotated Exports (Phase 3 completion)

Date: 2026-06-01
Status: Approved (brainstorming) — ready for implementation
Roadmap: completes Phase 3 ("Analysis Database") and "Recommended Next Slice #2".

## Goal

Wire the existing `AnalysisProject` (added in commit `aded090`) into the TUI so a
user can rename, comment, and bookmark addresses; see those annotations inline
(IDA-style); persist them (explicit `S` save + auto-save on clean quit); reload
them via `--project`; and have the export formats carry them. This satisfies all
Phase 3 acceptance criteria:

- User can rename a function, add a comment, save, restart, reload, and see the
  same analysis.
- Project files are versioned (already `schema_version: 1`) and documented.
- Exports include user names and comments.

## Current state (where we start)

- `src/project.rs`: `AnalysisProject` model (user_names, comments, bookmarks,
  functions), JSON `save`/`load`, and mutators `set_user_name`, `set_comment`,
  `toggle_bookmark`. Fully unit-tested.
- `src/main.rs`: `--project FILE` loads a project, `--save-project FILE` saves one
  (non-TUI path only). The loaded project is **not** passed to the TUI.
- `src/tui/`: `run_tui(instructions, cfg, analysis)` → `App::new(...)`. No project,
  no edit actions, no inline display.

## Non-goals (deferred to later phases)

- Bookmark label text (v1 toggles with no label).
- Renaming/commenting from the Names/Xrefs tabs (v1: Instructions + Functions).
- Undo/redo of edits.
- Refactoring the existing `search_mode` / `address_jump_mode` flags.

## Architecture

### Data flow / wiring

- `main.rs` builds the in-effect project: load `--project` if given, else
  `AnalysisProject::from_binary(file_path, &data)`. It computes a save path:
  `--save-project` → `--project` → default `<binary>.disproj.json`. Both the
  project and the save path are passed into the TUI.
- `run_tui(instructions, cfg, analysis, project, project_path)` →
  `App::with_project(...)`.
- `App::new(instructions, cfg, analysis)` is kept as a thin wrapper that builds an
  empty in-memory project with no path, so the existing 8 TUI unit tests and any
  test ergonomics stay intact.

### App state (new fields)

- `project: AnalysisProject`
- `project_path: Option<PathBuf>`
- `dirty: bool` — set on every edit, cleared on save.
- `prompt: Option<EditPrompt>` where
  `EditPrompt { kind: EditPromptKind, target: Address, input: String }` and
  `EditPromptKind { Rename, Comment }`. One enum captures the in-flight text-input
  modal and the address it targets (captured when the prompt opens, so commit is
  unambiguous). Chosen over mirroring the existing per-mode `bool`+`String` pairs
  for clarity. The existing search/jump modes are left untouched.
- `bookmarks_overlay: bool`, `bookmark_list_state: ListState`,
  `selected_bookmark: Option<usize>`.
- `name_by_address: HashMap<u64, String>`, `comment_by_address: HashMap<u64, String>`
  — O(1) lookup caches for the windowed instruction renderer; rebuilt by
  `refresh_annotation_caches()` at construction and after every edit.

### Actions (App methods, all unit-tested)

- `current_target_address() -> Option<Address>`: selected instruction address in
  the Instructions tab; selected function entry in the Functions tab; else `None`.
- `begin_rename()`, `begin_comment()`: open the prompt for the current target
  (pre-fill `input` with the existing name/comment if present); no-op + status hint
  on other tabs.
- `cancel_prompt()`, `prompt_push_char(c)`, `prompt_pop_char()`, `commit_prompt()`:
  commit applies to the project (`set_user_name`/`set_comment`, empty input clears),
  sets `dirty`, rebuilds caches, sets a status message.
- `toggle_bookmark_at_target()`: `project.toggle_bookmark(addr, None)`, sets dirty.
- `open_bookmarks()`, `close_bookmarks()`, `next_bookmark()`, `previous_bookmark()`,
  `jump_to_selected_bookmark()` (reuses `find_instruction_at_or_after` +
  `select_instruction` + switch to Instructions, recording history).
- `save_project() -> io::Result<()>`: writes to `project_path` if set, clears
  `dirty`, sets status; on error sets an error status (does not crash the TUI).
- `autosave_on_exit()`: if `dirty` and a path is set, save; ignore/Report errors.
- `name_for(u64) / comment_for(u64) / is_bookmarked(u64)`: read accessors used by
  views and exports (backed by `project.rs` accessors + the caches).

### Keybindings (free keys; `r`/`n`/`N`/`g`/`s`/`c`/`b?` checked against input.rs)

- `R` → rename prompt
- `;` → comment prompt
- `b` → toggle bookmark at target
- `B` → open/close bookmarks overlay
- `S` → save project
- Quit (`q`/`Esc`/`Ctrl-C`) → set a `should_quit` flag, break the loop, then
  `autosave_on_exit()`. (Refactor the three early `return Ok(())` branches into the
  flag so save happens exactly once.)
- While a prompt is open: chars append, Backspace pops, Enter commits, Esc cancels.
- While the bookmarks overlay is open: Up/Down select, Enter jumps + closes,
  Esc/`B` closes.

### Inline display (IDA-style)

- Instructions list: a named address renders an extra label line `00001000  name:`
  above the instruction row; a comment renders trailing ` ; comment` on the
  instruction line. ListItems already support multiple lines (Functions list does).
- Instruction Details panel: add "Name" and "Comment" rows when present.
- Functions list: show the user name after the entry address when present.
- Functions Details panel: add "Name" and "Comment" rows when present.
- Bookmarks overlay: centered popup (like Help) listing bookmarks sorted by
  address, each line `0xADDR  name/nearest-symbol`.
- Status bar: while a prompt is open, show `RENAME 0xADDR: <input>` /
  `COMMENT 0xADDR: <input>` with Enter/Esc hints; default bar mentions `R ; b B S`.
- Help overlay: document the new keys.

### Exports (annotations carried)

- `ExportableInstruction` gains `user_name: Option<String>` and
  `comment: Option<String>`, both `#[serde(skip_serializing_if = "Option::is_none")]`
  so existing JSON consumers are unaffected when absent.
- New `ExportAnnotations { names: HashMap<u64,String>, comments: HashMap<u64,String> }`
  with `ExportAnnotations::from_project(&AnalysisProject)`.
- Existing public export entry points gain thin `*_annotated` siblings taking
  `Option<&ExportAnnotations>`; the originals delegate with `None` (callers/tests
  preserved). A private core builds `ExportData` with annotated instructions.
- Renderers: CSV/Markdown/HTML gain Name + Comment columns; Assembly emits
  `name:` label lines + trailing `; comment`; JSON carries the per-instruction
  fields. DOT stays CFG-only.
- `main.rs` export branch builds `ExportAnnotations` from the in-effect project and
  calls the `*_annotated` entry points.

## Testing strategy (TDD — test first per unit)

- `project.rs`: `name_for`/`comment_for`/`is_bookmarked` accessors.
- `app.rs`: rename/comment set project + `dirty` + cache; empty input clears;
  `EditPrompt` lifecycle; `current_target_address` per tab; bookmark toggle add/
  remove; bookmarks jump; `save_project` round-trip (save to temp dir → reload →
  assert equal) — this is the headline "save, restart, reload, see same analysis"
  test at the App layer.
- `export.rs`: `ExportAnnotations::from_project`; a CSV export of an annotated
  instruction contains the comment text; a JSON `ExportableInstruction` serializes
  `user_name`. (Write to temp dir, read back, assert.)
- Views/input are thin wiring over tested App methods; verified by `cargo build`,
  `clippy`, and a manual smoke run on `tests/notepad.exe`.

## Build sequence (each step ends green: fmt + clippy + test)

1. `project.rs`: add read accessors (`name_for`, `comment_for`, `is_bookmarked`) + tests.
2. `app.rs`: add state fields, `with_project`, edit/bookmark/save/cache methods + tests.
3. `input.rs`: route new keys, prompt + overlay handling, `should_quit` + autosave.
4. `views.rs`: inline label/comment, details rows, function name, bookmarks overlay,
   status-bar prompt, help text.
5. `session.rs` + `main.rs`: thread `project` + `project_path` into `run_tui`;
   build/derive the project and save path; pass annotations to the export branch.
6. `export.rs`: annotated instructions, `ExportAnnotations`, `*_annotated` entries,
   renderer updates + tests.
7. Docs: README controls + `--project` round-trip example, ROADMAP Phase 3 →
   complete, optional `smoke_test.sh` project round-trip.
8. Final `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`,
   `cargo test`, manual smoke run; commit.

## Acceptance criteria (verified at the end)

- In the TUI: select an instruction, press `R`, type a name, Enter → label appears;
  press `;`, type a comment → trailing comment appears; press `b` → bookmark toggles
  and shows in `B` overlay; `S` saves; relaunch with `--project <file>` → annotations
  reappear.
- `cargo test` passes including the App-layer save→reload round-trip test.
- A CSV/JSON export of a project with annotations contains the name/comment.
- `cargo fmt --check` and `cargo clippy -D warnings` are clean.
