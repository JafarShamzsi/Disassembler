# DisAssembler Sharingan

### Description
The DeCompiler is a Rust-based binary analysis tool that provides disassembly, control flow analysis, and interactive visualization capabilities for PE (Portable Executable) files. It features a modern terminal user interface (TUI) built with ratatui for enhanced user experience.

### Key Features
- **Binary Parsing**: Extracts and analyzes `.text` sections from PE files
- **Disassembly**: Uses the iced_x86 library for accurate x86/x64 instruction disassembly
- **Control Flow Analysis**: Builds control flow graphs (CFGs) to understand program structure
- **Interactive TUI**: Modern terminal interface with multiple views and navigation
- **Multiple Output Modes**: Terminal output, TUI mode, and control flow visualization

### Technology Stack
- **Language**: Rust 🦀
- **Disassembly Engine**: `iced_x86` - High-performance x86/x64 disassembler
- **TUI Framework**: `ratatui` - Modern terminal user interface library
- **CLI Framework**: `clap` - Command-line argument parsing
- **Binary Parsing**: `scroll` - Binary parsing utilities
- **Cross-platform Terminal**: `crossterm` - Terminal manipulation

## Installation & Setup

### Prerequisites
- **Rust**: Version 1.70 or higher
- **Cargo**: Rust's package manager (included with Rust)
- **Git**: For cloning the repository

### Installation Steps

1. **Clone the Repository**
   ```bash
   git clone https://github.com/your-username/decompiler.git
   cd decompiler/main
   ```

2. **Build the Project**
   ```bash
   # Debug build (faster compilation, slower execution)
   cargo build
   
   # Release build (slower compilation, optimized execution)
   cargo build --release
   ```

3. **Run Tests**
   ```bash
   cargo test
   ```

### Dependencies
The project uses the following key dependencies:

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }  # CLI argument parsing
env_logger = "0.10"                                # Logging framework
iced_x86 = "1.21"                                 # x86/x64 disassembler
ratatui = "0.26"                                  # Terminal UI framework
crossterm = "0.27"                                # Cross-platform terminal
scroll = "0.11"                                   # Binary parsing utilities
goblin = "0.8"                                    # Binary format parsing
```

## Usage Guide

### Command Line Interface

#### Basic Usage
```bash
# Basic disassembly (terminal output)
cargo run -- /path/to/binary.exe

# Multiple files
cargo run -- file1.exe file2.exe file3.exe
```

#### Advanced Options
```bash
# Enable control flow graph analysis
cargo run -- --cfg /path/to/binary.exe

# Launch interactive TUI mode
cargo run -- --tui /path/to/binary.exe

# TUI with control flow analysis
cargo run -- --tui --cfg /path/to/binary.exe

# Raw output mode
cargo run -- --raw /path/to/binary.exe
```

#### Help Information
```bash
cargo run -- --help
```

### TUI Mode Features

#### Interface Layout
```
┌─────────────────── Decompiler ────────────────────┐
│ [Instructions] [Control Flow] [Hex Dump]         │
├───────────────────────────────────────────────────┤
│                                                   │
│  Main Content Area                                │
│  (Changes based on selected tab)                  │
│                                                   │
├───────────────────────────────────────────────────┤
│ Status: Instructions: 1234 | Selected: 5 | ...   │
└───────────────────────────────────────────────────┘
```

#### Keyboard Shortcuts

**Navigation:**
- `↑/k` - Previous instruction
- `↓/j` - Next instruction  
- `Tab` - Next tab
- `Shift+Tab` - Previous tab
- `1/2/3` - Direct tab selection

**General:**
- `h/F1` - Toggle help overlay
- `q` - Quit application

#### Tab Descriptions

1. **Instructions Tab**
   - Left panel: Scrollable list of disassembled instructions
   - Right panel: Detailed view of selected instruction
   - Shows address, mnemonic, operands, and raw bytes

2. **Control Flow Tab**
   - Displays control flow graph information
   - Shows basic blocks and their connections
   - Identifies jump targets and branch conditions

3. **Hex Dump Tab**
   - Raw hexadecimal view of instruction bytes
   - ASCII representation where applicable
   - Useful for low-level analysis

## Module Documentation

### main.rs
**Purpose**: Entry point and command-line interface handling

**Key Components:**
- `Opts` struct: Defines CLI arguments using clap
- `main()` function: Orchestrates the entire analysis pipeline
- Helper functions for instruction text parsing

**CLI Arguments:**
```rust
pub struct Opts {
    #[clap(long)]
    raw: bool,      // Raw output mode
    
    #[clap(long)]
    cfg: bool,      // Enable control flow graph
    
    #[clap(long)]
    tui: bool,      // Launch TUI mode
    
    #[clap(name = "FILE", value_parser)]
    files: Vec<PathBuf>,  // Input files
}
```

### parser.rs
**Purpose**: PE file parsing and .text section extraction

**Key Components:**
```rust
pub struct TextSection {
    pub va: u64,        // Virtual address
    pub bytes: Vec<u8>, // Section bytes
}

pub fn get_text_section(data: &[u8]) -> Result<TextSection, Box<dyn std::error::Error>>
```

**Functionality:**
- Parses PE headers using goblin library
- Locates and extracts .text section
- Handles various PE file formats (32-bit/64-bit)
- Error handling for malformed files

### disassembler.rs
**Purpose**: Assembly instruction disassembly using iced_x86

**Key Components:**
```rust
pub struct DisasmOpts {
    pub base_address: u64,  // Starting virtual address
    pub bitness: u32,       // Architecture (32 or 64-bit)
}

pub struct Instruction {
    pub address: u64,       // Instruction address
    pub bytes: Vec<u8>,     // Raw instruction bytes
    pub text: String,       // Formatted assembly text
}

pub fn disasm(bytes: &[u8], opts: DisasmOpts) -> Vec<Instruction>
```

**Features:**
- Uses iced_x86 for accurate disassembly
- Supports x86 and x64 architectures
- NASM syntax formatting
- Instruction boundary detection
- Raw byte extraction

### graph.rs
**Purpose**: Control flow graph construction and analysis

**Key Components:**
```rust
pub struct Address(pub u64);

pub struct Instruction {
    pub address: Address,
    pub mnemonic: String,
    pub operands: String,
    pub bytes: Vec<u8>,
}

pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub successors: Vec<Address>,
    pub predecessors: Vec<Address>,
}

pub enum EdgeType {
    Unconditional,      // Direct jumps
    ConditionalTrue,    // Branch taken
    ConditionalFalse,   // Branch not taken
    Call,              // Function calls
    Return,            // Return instructions
}

pub struct ControlFlowGraph {
    pub blocks: HashMap<Address, BasicBlock>,
    pub edges: Vec<Edge>,
}
```

**Analysis Process:**
1. **Basic Block Identification**: Finds instruction sequences with single entry/exit
2. **Edge Analysis**: Determines control flow between blocks
3. **Jump Target Resolution**: Identifies branch and call targets
4. **Graph Construction**: Builds connected graph structure

**Supported Instructions:**
- Conditional jumps: `je`, `jne`, `jl`, `jg`, `jle`, `jge`, etc.
- Unconditional jumps: `jmp`
- Function calls: `call`
- Returns: `ret`

### tui.rs
**Purpose**: Interactive terminal user interface

**Key Components:**
```rust
pub struct App {
    pub instructions: Vec<Instruction>,
    pub cfg: Option<ControlFlowGraph>,
    pub current_tab: Tab,
    pub instruction_list_state: ListState,
    pub selected_instruction: Option<usize>,
    pub show_help: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Instructions,
    ControlFlow,
    HexDump,
}
```

**UI Features:**
- Tabbed interface with three main views
- Keyboard navigation and shortcuts
- Context-sensitive help system
- Real-time instruction details
- Scrollable content areas

## API Reference

### Parser Module

#### `get_text_section(data: &[u8]) -> Result<TextSection, Box<dyn std::error::Error>>`
Extracts the .text section from PE file data.

**Parameters:**
- `data`: Raw PE file bytes

**Returns:**
- `Ok(TextSection)`: Successfully extracted section
- `Err(...)`: Parse error with description

**Example:**
```rust
use std::fs;
let data = fs::read("example.exe")?;
let text_section = parser::get_text_section(&data)?;
println!("Text section at VA: {:#x}", text_section.va);
```

### Disassembler Module

#### `disasm(bytes: &[u8], opts: DisasmOpts) -> Vec<Instruction>`
Disassembles binary code into assembly instructions.

**Parameters:**
- `bytes`: Raw instruction bytes
- `opts`: Disassembly options (address, bitness)

**Returns:**
- Vector of disassembled instructions

**Example:**
```rust
let opts = DisasmOpts {
    base_address: 0x401000,
    bitness: 64,
};
let instructions = disasm(&bytes, opts);
for inst in instructions {
    println!("{}", inst);
}
```

### Graph Module

#### `ControlFlowGraph::new() -> Self`
Creates a new empty control flow graph.

#### `build_from_instructions(&mut self, instructions: Vec<Instruction>)`
Builds CFG from a vector of instructions.

**Process:**
1. Identifies basic block boundaries
2. Creates basic blocks from instruction sequences
3. Analyzes control flow between blocks
4. Builds edge relationships

#### `display_ascii(&self)`
Prints ASCII art representation of the CFG.

#### `display_simple(&self)`
Prints simple text representation of the CFG.

### TUI Module

#### `run_tui(instructions: Vec<Instruction>, cfg: Option<ControlFlowGraph>) -> Result<(), Box<dyn std::error::Error>>`
Launches the interactive terminal user interface.

**Parameters:**
- `instructions`: Disassembled instructions
- `cfg`: Optional control flow graph

**Returns:**
- `Ok(())`: TUI exited successfully
- `Err(...)`: TUI error

## Examples

### Basic Disassembly
```rust
use std::fs;
use decompiler::{parser, disassembler::{disasm, DisasmOpts}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read binary file
    let data = fs::read("example.exe")?;
    
    // Extract .text section
    let text_section = parser::get_text_section(&data)?;
    
    // Configure disassembler
    let opts = DisasmOpts {
        base_address: text_section.va,
        bitness: 64,
    };
    
    // Disassemble instructions
    let instructions = disasm(&text_section.bytes, opts);
    
    // Print results
    for inst in instructions {
        println!("{:#08x}: {}", inst.address, inst.text);
    }
    
    Ok(())
}
```

### Control Flow Analysis
```rust
use decompiler::{graph::{ControlFlowGraph, Instruction as CfgInstruction, Address}};

fn analyze_control_flow(instructions: Vec<disassembler::Instruction>) {
    // Convert to CFG format
    let cfg_instructions: Vec<CfgInstruction> = instructions
        .iter()
        .map(|inst| CfgInstruction {
            address: Address(inst.address),
            mnemonic: extract_mnemonic(&inst.text),
            operands: extract_operands(&inst.text),
            bytes: inst.bytes.clone(),
        })
        .collect();
    
    // Build control flow graph
    let mut cfg = ControlFlowGraph::new();
    cfg.build_from_instructions(cfg_instructions);
    
    // Display results
    cfg.display_ascii();
}
```

### TUI Integration
```rust
use decompiler::tui;

fn launch_interactive_mode(
    instructions: Vec<disassembler::Instruction>,
    cfg: Option<ControlFlowGraph>
) -> Result<(), Box<dyn std::error::Error>> {
    tui::run_tui(instructions, cfg)
}
```

## Development Guide

### Building from Source

#### Debug Build
```bash
cargo build
```
- Faster compilation
- Includes debug symbols
- No optimizations
- Located at `target/debug/main`

#### Release Build
```bash
cargo build --release
```
- Slower compilation
- Optimized for performance
- Smaller binary size
- Located at `target/release/main`

### Testing

#### Run All Tests
```bash
cargo test
```

#### Run Specific Test
```bash
cargo test test_name
```

#### Run with Output
```bash
cargo test -- --nocapture
```

#### Integration Tests
Integration tests are located in the tests directory and use the `notepad.exe` file as a test binary.

### Code Style

#### Formatting
```bash
cargo fmt
```

#### Linting
```bash
cargo clippy
```

#### Documentation
```bash
cargo doc --open
```

### Adding New Features

#### Adding a New Display Mode
1. Extend the `Tab` enum in tui.rs
2. Add rendering function
3. Update tab navigation logic
4. Add keyboard shortcuts

#### Adding New Architecture Support
1. Update `DisasmOpts` to support new architecture
2. Modify disassembly logic in `disassembler.rs`
3. Update CFG analysis for architecture-specific instructions
4. Add tests for new architecture

#### Adding New Binary Formats
1. Extend parser logic in `parser.rs`
2. Add format detection
3. Implement section extraction for new format
4. Update error handling

### Performance Considerations

#### Memory Usage
- Large binaries may consume significant memory
- Consider streaming for very large files
- CFG construction can be memory-intensive

#### Optimization Tips
- Use release builds for large files
- Consider limiting disassembly to specific sections
- Implement lazy loading for TUI

## Troubleshooting

### Common Issues

#### "No files provided" Error
**Problem**: Running without specifying input files
**Solution**: Provide at least one PE file as argument
```bash
cargo run -- example.exe
```

#### "Failed to parse .text" Error
**Problem**: Invalid PE file or unsupported format
**Solutions:**
- Verify file is a valid PE executable
- Check file permissions
- Ensure file is not corrupted

#### TUI Not Responding
**Problem**: Terminal compatibility issues
**Solutions:**
- Ensure terminal supports ANSI escape codes
- Try different terminal emulator
- Check crossterm compatibility

#### Large File Performance
**Problem**: Slow processing of large binaries
**Solutions:**
- Use release build: `cargo build --release`
- Limit analysis to specific sections
- Increase system memory if available

### Debug Mode

#### Enable Debug Logging
```bash
RUST_LOG=debug cargo run -- example.exe
```

#### Verbose Output
```bash
RUST_LOG=trace cargo run -- example.exe
```

### System Requirements

#### Minimum Requirements
- RAM: 4GB (8GB recommended for large files)
- Storage: 100MB for source + build artifacts
- CPU: Any modern x86_64 processor

#### Terminal Requirements
- ANSI escape code support
- UTF-8 character encoding
- Minimum 80x24 character display

## Contributing

### Getting Started
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Guidelines
- Follow Rust naming conventions
- Add documentation for public APIs
- Include unit tests for new functions
- Update this documentation for major changes

### Submitting Issues
When reporting bugs, please include:
- Operating system and version
- Rust version (`rustc --version`)
- Complete error message
- Minimal reproduction case
- Sample binary file (if applicable)

### Feature Requests
For new features, please provide:
- Clear description of the feature
- Use case and motivation
- Proposed API or interface
- Consideration of implementation complexity

