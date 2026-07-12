# Rust Binary Disassembler with Graph View

A modern, feature-rich binary disassembler written in Rust with an interactive TUI and control flow graph visualization.

See [ROADMAP.md](ROADMAP.md) for the plan to evolve this into a more complete terminal-native reverse engineering workbench.

## Features

### **Disassembly Engine**
- **Architecture detection**: Detects x86, x86-64, ARM, AArch64, MIPS, and RISC-V metadata from PE/ELF headers
- **Disassembly support**: x86/x86-64 via iced-x86; ARM/AArch64 via Capstone
- **Binary format parsing**: PE and ELF section loading via goblin, with `.text` default and optional all-executable-section disassembly
- **Instruction analysis**: Comprehensive instruction decoding using iced-x86
- **Section analysis**: PE/ELF section tables with virtual ranges, file offsets, sizes, permissions, and VA-to-file-offset mapping

### **Control Flow Graph (CFG)**
- **Advanced block detection**: Smart basic block identification with jump/call target analysis and conservative noreturn import fallthrough suppression
- **Graph metrics**: Cyclomatic complexity, branching factor, block statistics
- **Call graph analysis**: Function-level caller/callee graph with resolved internal calls, external import call sites, import-thunk classification/naming, entry/symbol/export/unwind-backed function confidence metadata, and JSON/HTML/Markdown export metadata
- **Multiple layouts**: Grid layout for large graphs with viewport navigation
- **Export support**: JSON, CSV, HTML, Markdown, DOT, and Assembly formats

### **Interactive TUI**
- **Multi-tab interface**: Overview, Instructions, Functions, Call Graph, Imports, Exports, Symbols, Names, Strings, Data, Relocations, Sections, Xrefs, Bookmarks, Control Flow, Graph View, Hex Dump
- **Overview dashboard**: First-screen binary orientation with file identity, format, entry point, analysis counts, project state, and section mix
- **Function browser**: Filtered inferred functions with kind/confidence labels, caller counts, and jump-to-entry navigation
- **Call graph browser**: Function-level incoming/outgoing call counts, internal/external import callees, resolved import-thunk targets, caller/callee details, kind/confidence labels, and jump-to-entry navigation
- **Imports browser**: Imported libraries and IAT entries with incoming xrefs, file offsets, Hex Dump handoff, and first-referrer navigation
- **Exports browser**: Exported symbols and forwarders with kind labels, sizes, section/file-offset context, xrefs, and code/Hex navigation
- **Symbols browser**: Parsed binary symbols with kind labels, section/file-offset context, xrefs, and code/Hex navigation
- **Names browser**: Imports, symbols, and user-defined names with address-oriented jump navigation
- **Strings browser**: Printable strings from file-backed data sections with section labels, byte preview, xrefs, and Hex Dump handoff
- **Data browser**: Decoded data-section pointer objects with target labels, outgoing xrefs, Hex Dump handoff, and executable-target following
- **Relocations browser**: PE base/COFF and ELF dynamic/PLT/section relocations with source/type/symbol/addend details and Hex/instruction navigation
- **Sections browser**: PE/ELF sections with permissions, virtual address ranges, file offsets, and jump-to-section navigation
- **Cross-reference browser**: CFG-backed call/jump references plus import, symbol, string, and data-pointer references with source navigation
- **Analysis workspace**: Rename addresses, add comments, toggle bookmarks, and persist them in project files
- **Graph navigation**: Arrow key navigation between connected blocks with lazy whole-program layout, function-scoped graph mode, and Enter-to-instruction synchronization
- **Search functionality**: Fast instruction search with filtering
- **Viewport controls**: Pan (WASD), zoom (+/-), center (C)
- **Mapped hex view**: Section-backed VA/file-offset hex browser synchronized with the selected address

### **Analysis Features**
- **Basic block analysis**: Block and edge analysis for control-flow exploration, with known noreturn imports used to avoid false call fallthrough edges
- **Names analysis**: Imports, exports, debug symbols, data-section printable strings, and decoded data pointers for quick orientation
- **Named xrefs**: Instruction operand scanning links code to import slots, symbols, and string ranges when direct addresses are present; data pointers add section-to-target xrefs; relocation rows expose loader patch sites
- **Unwind function recovery**: PE x64 exception/runtime function ranges are parsed, surfaced with `--unwind`, and used as high-confidence CFG function seeds
- **Loop detection**: Experimental back-edge based loop identification
- **Metrics calculation**: Comprehensive graph statistics
- **Call graph**: Function-level call graph for browsing and exported reports, with external import call edges, resolved import-thunk wrappers, noreturn-aware CFG edges, and higher-confidence internal functions seeded by entry points, executable symbols/exports, PE unwind ranges, calls, and prologues
- **Planned analysis**: Mach-O support, deeper data-flow xrefs, and unreachable block detection

## Quick Start

### Prerequisites
- Rust 1.70+
- Linux/macOS/Windows

### Installation
```bash
git clone https://github.com/yourusername/disassembler.git
cd disassembler
cargo build --release
```

### Usage

#### Command Line Interface
```bash
# Basic disassembly
./target/release/disassembler binary.exe

# Binary orientation and control flow graph metrics
./target/release/disassembler binary.exe --overview
./target/release/disassembler binary.exe --cfg --metrics

# Imports, exports, symbols, strings, sections, data pointers, relocations, unwind ranges, and calls
./target/release/disassembler binary.exe --names
./target/release/disassembler binary.exe --imports
./target/release/disassembler binary.exe --exports
./target/release/disassembler binary.exe --symbols
./target/release/disassembler binary.exe --sections
./target/release/disassembler binary.exe --data
./target/release/disassembler binary.exe --relocations
./target/release/disassembler binary.exe --unwind
./target/release/disassembler binary.exe --callgraph

# Disassemble executable sections beyond the default .text view
./target/release/disassembler binary.exe --all-executable --functions
./target/release/disassembler binary.exe --section .text --cfg
./target/release/disassembler binary.exe --section 0x140001000 --cfg

# Save or reload user analysis state
./target/release/disassembler binary.exe --functions --save-project analysis.disproj.json
./target/release/disassembler binary.exe --project analysis.disproj.json --names

# Persist TUI renames, comments, and bookmarks
./target/release/disassembler binary.exe --tui --save-project analysis.disproj.json
./target/release/disassembler binary.exe --tui --project analysis.disproj.json

# Export analysis
./target/release/disassembler binary.exe --cfg --output analysis.json --format json
```

#### Interactive TUI
```bash
./target/release/disassembler binary.exe --tui
```
#### Screenshots

<img width="1917" height="1064" alt="Screenshot From 2025-08-03 22-55-22" src="https://github.com/user-attachments/assets/b6f62fcb-faf8-41e7-9b75-7ec53cdfe468" />
<img width="1917" height="1064" alt="Screenshot From 2025-08-03 23-20-28" src="https://github.com/user-attachments/assets/b56dcafc-b54c-40b8-9d8c-2a51eaa9771f" />
<img width="1917" height="1064" alt="Screenshot From 2025-08-03 23-20-38" src="https://github.com/user-attachments/assets/1cb32bdf-e6f3-4e28-8fb0-432c91811873" />
<img width="1917" height="1064" alt="Screenshot From 2025-08-03 23-20-47" src="https://github.com/user-attachments/assets/c6e7304b-1b30-4675-90f1-c48f916d2dc9" />
<img width="1917" height="1064" alt="Screenshot From 2025-08-03 23-21-20" src="https://github.com/user-attachments/assets/00474c98-1a22-4c93-9b55-82c5fde9b2dd" />


**TUI Controls:**
- `Tab`: Switch between views
- `O`: Jump to Overview
- `1`: Jump to Instructions
- `2`: Jump to Functions
- `3`: Jump to Call Graph
- `4`: Jump to Names
- `I`: Jump to Imports
- `E`: Jump to Exports
- `Y`: Jump to Symbols
- `S`: Jump to Strings
- `D`: Jump to Data
- `L`: Jump to Relocations
- `5`: Jump to Sections
- `6`: Jump to Xrefs
- `7`: Jump to Bookmarks
- `8`: Jump to Control Flow
- `9`: Jump to Graph View
- `0`: Jump to Hex Dump
- `Enter`: Jump from Overview to the entry point, or from the selected function, call graph function, import, export, symbol, name, string, data pointer, relocation, section, xref source, bookmark, or graph block
- `g`: Jump to an address
- `u` / `r`: Navigation back / forward
- `R`: Rename the selected address
- `;`: Add or edit a comment at the selected address
- `b`: Toggle a bookmark at the selected address
- `n` / `N`: Next / previous search result
- `Arrow Keys`: Navigate graph/instructions
- `WASD`: Pan viewport
- `+/-`: Zoom
- `C`: Center on selection
- `f`: Toggle Graph View between whole-program and current-function scope
- `/`: Search
- `h`: Help
- `q`: Quit

#### Safe Execution (Recommended)
```bash
# Use wrapper script for enhanced terminal safety
./scripts/safe_run.sh tests/notepad.exe --tui
```

## Architecture

### Core Components
- **`src/main.rs`**: CLI interface and application entry point
- **`src/parser.rs`**: Binary format parsing, executable/data section loading, section tables, VA-to-file-offset mapping, imports/symbols, PE unwind ranges, string extraction, and data-pointer summaries
- **`src/arch/`**: Architecture-specific disassembly engines
- **`src/graph.rs`**: Control flow graph construction, seeded function summaries, and call graph analysis using petgraph
- **`src/graph_view.rs`**: Lazy graph layout, function-scoped graph filtering, and viewport management
- **`src/tui/`**: Interactive terminal user interface, split into app state, input, session, and views modules
- **`src/export.rs`**: Multi-format export functionality

### Dependencies
- **`iced-x86`**: Fast x86/x86-64 disassembler
- **`goblin`**: Binary format parsing
- **`petgraph`**: Graph algorithms and data structures
- **`ratatui`**: Terminal user interface framework
- **`clap`**: Command line argument parsing

## Examples

### CFG Metrics Output
```
[METRICS] Graph Metrics:
+-----------------------------+-------------+
| Metric                      | Value       |
+-----------------------------+-------------+
| Total Blocks                |        8010 |
| Total Edges                 |        6600 |
| Cyclomatic Complexity       |           2 |
| Average Block Size          |        5.50 |
| Branching Factor            |        0.82 |
+-----------------------------+-------------+
```

### Graph View Features
- **Multi-block visualization**: Proper basic block splitting
- **Edge styling**: Color-coded edges (conditional, unconditional, calls)
- **Block details**: Instruction listing and successor/predecessor info
- **Navigation**: Seamless movement between connected blocks, with Enter returning from a graph block to the instruction list
- **Function scope**: Toggle Graph View between whole-program and current-function blocks for large binaries

## Development

### Building
```bash
cargo build          # Debug build
cargo build --release # Optimized build
cargo test           # Run tests
```

### Testing
```bash
# Test with sample binary
./target/debug/disassembler tests/notepad.exe --cfg --metrics

# Run the full local smoke suite
./scripts/smoke_test.sh

# Interactive testing
./target/debug/disassembler tests/notepad.exe --tui
```

## Troubleshooting

### Terminal Issues
If you experience terminal corruption or ANSI escape sequences:

1. **Emergency reset**: `./scripts/emergency_reset.sh`
2. **Use safe wrapper**: `./scripts/safe_run.sh` instead of direct execution
3. **Manual reset**: `reset && clear`

### Common Issues
- **Build errors**: Ensure a supported Rust toolchain is installed
- **Binary parsing**: Check file format compatibility
- **Performance**: Use `--release` build for large binaries

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for local setup, PR expectations, and TUI development notes.

Quick checklist:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Add tests if applicable
5. Run `cargo fmt --check`, `cargo test --all-targets --all-features`, and `cargo clippy --all-targets --all-features -- -D warnings`
6. Submit a pull request

Security issues should follow [SECURITY.md](SECURITY.md).

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Add documentation for public APIs
- Include tests for new features

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **iced-x86**: Excellent x86 disassembly engine
- **petgraph**: Robust graph algorithms
- **ratatui**: Modern terminal UI framework
- **goblin**: Comprehensive binary parsing
