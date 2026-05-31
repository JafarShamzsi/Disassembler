# Rust Binary Disassembler with Graph View

A modern, feature-rich binary disassembler written in Rust with an interactive TUI and control flow graph visualization.

## Features

### **Disassembly Engine**
- **x86/x86-64 support**: x86/x86-64 disassembly via iced-x86
- **Extensible architecture framework**: ARM/AArch64 disassembler module is present but not yet wired into the CLI
- **Binary format parsing**: PE `.text` extraction via goblin
- **Instruction analysis**: Comprehensive instruction decoding using iced-x86
- **Section analysis**: Automatic .text section extraction and analysis

### **Control Flow Graph (CFG)**
- **Advanced block detection**: Smart basic block identification with jump/call target analysis
- **Graph metrics**: Cyclomatic complexity, branching factor, block statistics
- **Multiple layouts**: Grid layout for large graphs with viewport navigation
- **Export support**: JSON, CSV, HTML, Markdown, DOT, and Assembly formats

### **Interactive TUI**
- **Multi-tab interface**: Instructions, Control Flow, Graph View, Hex Dump
- **Graph navigation**: Arrow key navigation between connected blocks
- **Search functionality**: Fast instruction search with filtering
- **Viewport controls**: Pan (WASD), zoom (+/-), center (C)

### **Analysis Features**
- **Basic block analysis**: Block and edge analysis for control-flow exploration
- **Loop detection**: Experimental back-edge based loop identification
- **Metrics calculation**: Comprehensive graph statistics
- **Planned analysis**: Function discovery, call graph, ELF/Mach-O support, and unreachable block detection

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

# Control flow graph with metrics
./target/release/disassembler binary.exe --cfg --metrics

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
- `3`: Jump to Graph View
- `Arrow Keys`: Navigate graph/instructions
- `WASD`: Pan viewport
- `+/-`: Zoom
- `C`: Center on selection
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
- **`src/parser.rs`**: Binary format parsing and section extraction
- **`src/arch/`**: Architecture-specific disassembly engines
- **`src/graph.rs`**: Control flow graph construction using petgraph
- **`src/graph_view.rs`**: Graph layout and viewport management
- **`src/tui.rs`**: Interactive terminal user interface
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
- **Navigation**: Seamless movement between connected blocks

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

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

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
