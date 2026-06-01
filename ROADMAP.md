# Roadmap: Toward a TUI Version of IDA

This roadmap is based on the current project state on 2026-06-01. The project already has PE/ELF parsing, x86/x64 and ARM/AArch64 disassembly, CFG construction, function summaries, names analysis, exports, and a Ratatui interface with staged work for Names and Xrefs browsing.

The goal is not to clone every IDA feature. The goal is to make this a genuinely usable terminal-native reverse engineering workbench: fast orientation, reliable navigation, editable analysis state, and exports that preserve discoveries.

## Current State

Implemented:
- Binary loading for PE and ELF with metadata, entry point, architecture, bitness, and endianness.
- `.text` extraction and disassembly for x86/x64 through `iced-x86`, ARM/AArch64 through Capstone.
- CFG construction with basic blocks, branch/call edges, graph metrics, function inference, and loop heuristics.
- Names analysis for imports, exported/debug symbols, and printable strings.
- TUI tabs for instructions, functions, graph/control-flow views, hex dump, and staged Names/Xrefs browsers.
- Export formats: JSON, CSV, HTML, Markdown, DOT, and assembly.

Main gaps:
- TUI responsibilities are split across focused modules, but `src/tui/views.rs` is still large and should eventually move to per-view files.
- Function recovery is heuristic and noisy; many one-instruction functions are inferred.
- Xrefs are CFG-edge based only; data references, import references, and string references are not yet found.
- No persistent analysis database for user names, comments, bookmarks, or patches.
- TUI terminal lifecycle is aggressive and uses `process::exit`, which is risky for a real tool.
- Large binaries can be slow because CFG and graph views are built eagerly.
- Open-source project hygiene is incomplete: no roadmap until this file, no issue templates, no contribution guide depth, no CI, no release artifacts.

## Phase 0: Stabilize The Baseline

Status: complete as of commit `6b7abff`.

Purpose: make the current staged work easy to trust before building more UI.

Work:
- Commit the staged Names and Xrefs browser changes once git commit escalation is available.
- Re-run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`.
- Add a short smoke-test script or documented command set for `tests/notepad.exe`.

Acceptance criteria:
- Working tree is clean after commit.
- Tests and clippy pass.
- README documents the current tab order accurately.

## Phase 1: TUI Architecture Split

Status: complete.

Purpose: make future IDA-like features cheaper and safer to add.

Work:
- Split `src/tui.rs` into focused modules:
  - `tui/app.rs` for state and navigation actions.
  - `tui/input.rs` for key handling.
  - `tui/views/` for instructions, functions, names, xrefs, graph, hex, help, and status.
  - `tui/session.rs` for terminal setup/cleanup.
- Replace forced `process::exit` in TUI flow with RAII terminal cleanup.
- Keep public `run_tui(...)` stable while internals move.

Acceptance criteria:
- No behavior regression in existing TUI unit tests.
- A panic or normal quit restores terminal state without forced process exit.
- Each TUI view can be understood without reading unrelated rendering code.

## Phase 2: Real Navigation Model

Status: in progress. Navigation history, address jump, and address context panels are implemented; cross-view search traversal is in progress.

Purpose: make the TUI feel like a reverse engineering tool instead of separate lists.

Work:
- Add a central navigation history stack: jump back/forward between instructions, functions, names, xrefs, and graph blocks.
- Add `g` / address jump, `n` / `N` search-next behavior, and selected-address synchronization across tabs.
- Add address-aware context panels: incoming xrefs, outgoing xrefs, containing function, nearest symbol, and bytes.
- Make search work across instructions, function entries, names, strings, and xrefs.

Acceptance criteria:
- Jumping from any navigable item records history.
- Back/forward returns to prior selections.
- Selecting an address in one tab can be reflected in related tabs.

## Phase 3: Analysis Database

Status: in progress. Project save/load foundations, core TUI annotation actions, inline annotation display, and annotated exports are implemented; bookmark list UI remains.

Purpose: support the core IDA workflow: rename, comment, bookmark, save, reload.

Work:
- Add an `AnalysisProject` model containing:
  - user-defined names,
  - comments,
  - bookmarks,
  - function metadata,
  - selected binary fingerprint/path.
- Add JSON project save/load.
- Add TUI actions for rename, comment, bookmark, and bookmark list.
- Display user names/comments inline in instruction and function views.
- Add annotated export support for names and comments.

Acceptance criteria:
- User can rename a function, add a comment, save, restart, reload, and see the same analysis.
- Project files are versioned and documented.
- Exports include user names and comments.

## Phase 4: Better Function And Xref Recovery

Purpose: improve analysis quality enough for real binaries.

Work:
- Recover functions from entry point traversal, symbols, exports, call targets, prologues, exception/unwind metadata where available, and import thunks.
- Classify functions as user code, thunk, import wrapper, library/runtime, or unknown.
- Add data xrefs by scanning instruction operands for addresses inside known sections.
- Add string xrefs and import call-site xrefs.
- Build a call graph separate from the low-level CFG.

Acceptance criteria:
- Function list is less noisy on `tests/notepad.exe`.
- Import and string entries show real referring instruction addresses where possible.
- Call graph can be exported and browsed independently from the CFG.

## Phase 5: Loader And Section Model

Purpose: stop treating `.text` as the whole binary.

Work:
- Add section/segment tables for PE and ELF.
- Add loaded-image address mapping from virtual address to file offset.
- Support disassembling selected executable sections, not just `.text`.
- Add data/rodata/rdata views.
- Add relocation, import table, export table, and symbol table views.

Acceptance criteria:
- TUI can browse sections and jump between section data and disassembly.
- Address-to-file-offset mapping is available to all analysis code.
- Strings are extracted from relevant data sections instead of every named section indiscriminately.

## Phase 6: Graph Usability

Purpose: make graph view useful on real programs.

Work:
- Add function-scoped graph mode.
- Add minimap or viewport overview.
- Add collapse/expand for large blocks and external calls.
- Add selected-node synchronization with instruction/function tabs.
- Avoid eagerly laying out enormous whole-program graphs.

Acceptance criteria:
- Opening a large binary does not block on full-program graph layout.
- A selected function can be graphed independently.
- Graph selection can jump to instruction and back.

## Phase 7: Decompiler-Like Aids

Purpose: provide high-value analysis without promising a full decompiler too early.

Work:
- Add stack variable and local frame hints for common x86/x64 prologues.
- Add branch condition summaries.
- Add pseudo-code skeletons from CFG structure: function header, basic block labels, calls, returns.
- Add import-aware call annotations.

Acceptance criteria:
- Function detail panel can show arguments/local hints when detected.
- Pseudo-code skeleton export is clearly labeled as experimental.
- No generated pseudo-code is presented as verified source.

## Phase 8: Open Source Readiness

Purpose: make the project usable and approachable outside the local workspace.

Work:
- Add CI for fmt, test, clippy, and release builds.
- Add `CONTRIBUTING.md`, issue templates, PR template, and security policy.
- Add a real project name/positioning, screenshots matching the current UI, and installation instructions.
- Add sample binaries or instructions for safe test inputs.
- Add release packaging for Linux/macOS/Windows.

Acceptance criteria:
- A new contributor can clone, test, and understand the roadmap in under 10 minutes.
- CI protects the main branch.
- README screenshots and controls match the current app.

## Phase 9: Performance And Robustness

Purpose: keep the tool responsive as features grow.

Work:
- Add lazy analysis jobs for CFG, strings, xrefs, and graph layout.
- Add progress states in TUI for expensive work.
- Add analysis limits and cancellation.
- Add benchmark fixtures for small, medium, and large binaries.
- Replace panics/unwraps in production paths with contextual errors.

Acceptance criteria:
- Large binaries stay responsive enough to quit, switch views, or cancel analysis.
- Benchmark results are tracked in CI or documented locally.
- User-facing errors include file path and failing operation.

## Recommended Next Three Implementation Slices

1. Add navigation history and address jump (`g`) because it immediately improves the IDA-like workflow.
2. Add persistent analysis state for user names, comments, and bookmarks.
3. Improve xrefs with string references, import call-site references, and data references.

These slices build on the completed TUI module split and should stay small enough to validate independently.
