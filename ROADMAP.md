# Roadmap: Toward a TUI Version of IDA

This roadmap is based on the current project state on 2026-06-01. The project already has PE/ELF parsing, x86/x64 and ARM/AArch64 disassembly, CFG construction, function summaries, names analysis, exports, and a Ratatui interface with staged work for Names and Xrefs browsing.

The goal is not to clone every IDA feature. The goal is to make this a genuinely usable terminal-native reverse engineering workbench: fast orientation, reliable navigation, editable analysis state, and exports that preserve discoveries.

## Current State

Implemented:
- Binary loading for PE and ELF with metadata, entry point, architecture, bitness, endianness, and section tables.
- `.text` default extraction plus optional selected/all executable-section disassembly for x86/x64 through `iced-x86`, ARM/AArch64 through Capstone.
- CFG construction with basic blocks, branch/call edges, conservative noreturn import fallthrough suppression, graph metrics, entry/symbol/export/unwind-backed filtered function inference with kind/confidence metadata, function-level call graph analysis, and loop heuristics.
- Names analysis for imports, exported/debug symbols, printable strings, and decoded data pointers.
- TUI tabs for overview, instructions, functions, call graph, imports, symbols, names, strings, data, sections, xrefs, bookmarks, graph/control-flow views, and mapped hex dump.
- Export formats: JSON, CSV, HTML, Markdown, DOT, and assembly.

Main gaps:
- TUI responsibilities are split across focused modules, but `src/tui/views.rs` is still large and should eventually move to per-view files.
- Function recovery is still heuristic; entry points, executable symbols/exports, PE unwind/runtime ranges, direct call targets, x86 frame-pointer prologues, import-thunk wrappers, and noreturn import hints improve CFG/call graph views, but broader prologue and tail-call recovery remain.
- Xrefs include CFG edges plus direct operand references to imports, symbols, strings, and decoded data pointers; deeper data-flow xrefs are not yet recovered.
- Project state now persists user names, comments, bookmarks, and function entries; richer function metadata and patch tracking remain.
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

Status: in progress. Overview landing, navigation history, address jump, address context panels, and cross-view search traversal are implemented; deeper synchronized multi-pane navigation remains.

Purpose: make the TUI feel like a reverse engineering tool instead of separate lists.

Work:
- Add a central navigation history stack: jump back/forward between instructions, functions, names, xrefs, and graph blocks.
- Add `g` / address jump, `n` / `N` search-next behavior, and selected-address synchronization across tabs.
- Add address-aware context panels: incoming xrefs, outgoing xrefs, containing function, nearest symbol, and bytes.
- Make search work across overview metadata, instructions, function entries, names, strings, data objects, sections, bookmarks, and xrefs. Implemented for current TUI datasets.

Acceptance criteria:
- Jumping from any navigable item records history.
- Back/forward returns to prior selections.
- Selecting an address in one tab can be reflected in related tabs.

## Phase 3: Analysis Database

Status: in progress. Project save/load foundations, TUI rename/comment/bookmark commands, bookmark browsing, and TUI project save/load wiring are implemented.

Purpose: support the core IDA workflow: rename, comment, bookmark, save, reload.

Work:
- Add an `AnalysisProject` model containing:
  - user-defined names,
  - comments,
  - bookmarks,
  - function metadata,
  - selected binary fingerprint/path.
- Add JSON project save/load.
- Add TUI actions for rename, comment, bookmark, and bookmark list. Implemented for address-level analysis.
- Display user names/comments inline in instruction and function views. Implemented for user names, comments, and bookmark markers.

Acceptance criteria:
- User can rename an address, add a comment, toggle bookmarks, save, restart, reload, and see the same analysis.
- Project files are versioned and documented.
- Exports include user names and comments.

## Phase 4: Better Function And Xref Recovery

Status: in progress. Direct import, symbol, string operand xrefs, decoded data-pointer xrefs, symbol/export/unwind-backed filtered function summaries with kind/confidence metadata, external import call edges, import-thunk classification/naming, conservative noreturn import fallthrough suppression, and a function-level call graph browser/export are implemented; deeper data-flow recovery remains.

Purpose: improve analysis quality enough for real binaries.

Work:
- Recover functions from entry point traversal, symbols, exports, call targets, prologues, exception/unwind metadata where available, and import thunks. Implemented for metadata entry-point seeds, executable symbol/export seeds, PE x64 unwind/runtime ranges, direct internal call targets, x86 frame-pointer prologues, and import-aware jump/call-return thunk wrappers.
- Classify functions as user code, thunk, import wrapper, library/runtime, or unknown. Implemented as entry/standard/thunk/unknown with confidence labels; richer source/kind taxonomy remains.
- Add data xrefs by scanning instruction operands and data sections for addresses inside known sections. Direct named-target operand scanning is implemented for imports, symbols, and strings; decoded pointer objects now add data-to-target xrefs.
- Add string xrefs and import call-site xrefs. Direct string/import target references, external import call graph edges, and import-thunk target labels are implemented.
- Build a call graph separate from the low-level CFG. Implemented for internal direct calls, external import call edges, resolved import-thunk wrappers, and noreturn-aware CFG edges with CLI, TUI browsing, and JSON/HTML/Markdown export metadata.

Acceptance criteria:
- Function list is less noisy on `tests/notepad.exe` by hiding low-confidence one/two-instruction call targets while keeping them as internal boundaries; executable symbols, exports, and PE unwind ranges create high-confidence boundaries when present.
- Import and string entries show real referring instruction addresses where direct operands expose those addresses.
- Call graph can be exported and browsed independently from the CFG. Implemented for internal direct calls, external import call edges, resolved import-thunk wrappers, and noreturn-aware fallthrough suppression.

## Phase 5: Loader And Section Model

Status: in progress. PE/ELF section summaries, `--overview`, `--imports`, `--exports`, `--symbols`, `--relocations`, `--unwind`, `--sections`, selected/all executable-section disassembly, TUI Overview, Imports, Exports, Symbols, Relocations, Sections, Strings, and Data tabs, section/data/import/export/symbol/relocation search, jump-to-section-start, VA-to-file-offset mapping, data-section string extraction, decoded pointer objects, relocation tables, PE unwind range tables, and section-backed Hex tab bytes are implemented; deeper relocation semantics remain.

Purpose: stop treating `.text` as the whole binary.

Work:
- Add section/segment tables for PE and ELF. Implemented as section summaries with VA ranges, file offsets, sizes, and permissions.
- Add loaded-image address mapping from virtual address to file offset. Implemented for file-backed section addresses.
- Support disassembling selected executable sections, not just `.text`. Implemented with `--section <NAME_OR_VA>`, `--all-executable`, and TUI all-executable loading.
- Add data/rodata/rdata views. Implemented for data-section strings and pointer objects; richer typed scalars and table views remain.
- Add relocation, import table, export table, symbol table, and unwind function-range views. Import, export, symbol, and relocation tables are implemented in CLI/TUI; PE unwind ranges are implemented in CLI; deeper relocation classification remains.

Acceptance criteria:
- TUI opens on an overview dashboard, can browse sections, and can jump from overview/section entries into disassembly when decoded instructions are available.
- Address-to-file-offset mapping is available through `BinaryAnalysis` and used by the TUI Hex tab.
- Strings are extracted from relevant file-backed non-executable data sections instead of every named section indiscriminately. Implemented with section labels in CLI/TUI.
- Data pointers are decoded from data-like sections, surfaced through --data, searchable in the TUI Data tab, and linked into xrefs.
- Imported libraries and IAT entries are surfaced through --imports, searchable in the TUI Imports tab, and linked to first referrers or Hex Dump.
- Exported symbols and forwarders are surfaced through --exports, searchable in the TUI Exports tab, and linked to executable code or mapped bytes.
- Relocation entries are surfaced through --relocations, searchable in the TUI Relocations tab, and linked to containing instructions or mapped bytes.
- Parsed symbols are surfaced through --symbols, searchable in the TUI Symbols tab, and linked to executable code or mapped bytes.

## Phase 6: Graph Usability

Status: in progress. Lazy graph layout, function-scoped graph mode, and graph-to-instruction jump synchronization are implemented; minimap and collapse/expand remain.

Purpose: make graph view useful on real programs.

Work:
- Add function-scoped graph mode. Implemented with Graph View `f` toggle using the selected/current function.
- Add minimap or viewport overview.
- Add collapse/expand for large blocks and external calls.
- Add selected-node synchronization with instruction/function tabs. Implemented for instruction-to-graph selection and graph-block-to-instruction jumps; direct function-tab graph open can still be refined.
- Avoid eagerly laying out enormous whole-program graphs. Implemented for TUI graph layout; CFG construction itself is still eager.

Acceptance criteria:
- Opening a large binary does not block on full-program graph layout. Implemented for graph rendering startup.
- A selected function can be graphed independently. Implemented.
- Graph selection can jump to instruction and back. Implemented.

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

Status: in progress. CI, contribution guide, security policy, issue templates, and PR template are implemented; release packaging and refreshed screenshots remain.

Purpose: make the project usable and approachable outside the local workspace.

Work:
- Add CI for fmt, test, clippy, and release builds. Implemented for Linux, Windows, and macOS.
- Add `CONTRIBUTING.md`, issue templates, PR template, and security policy. Implemented.
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

1. Add richer typed scalar, relocation, import/export table, and symbol table views so the loader model goes beyond decoded pointers and strings.
2. Add collapse/expand controls for large graph nodes and a minimap/viewport overview.
3. Add broader prologue, tail-call, and noreturn interprocedural recovery so the function list stays cleaner on optimized binaries.

These slices build on the completed TUI module split and should stay small enough to validate independently.
