use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use clap::{Arg, Command};

// Clean module declarations
pub mod arch;
pub mod parser;
pub mod graph;
pub mod tui;
pub mod export;

pub use crate::arch::x86::Instruction;
// Simple, clean imports
use arch::x86::{DisasmOpts, disasm};  // Use the legacy functions directly
use parser::TextSection;
use graph::{ControlFlowGraph, Instruction as CfgInstruction, Address};
use export::{Exporter, ExportFormat, export_auto_format};

#[derive(Debug, Clone)]
pub struct Opts {
    pub raw: bool,
    pub cfg: bool,
    pub tui: bool,

    /// Export to file (format auto-detected from extension)
    pub output: Option<PathBuf>,

    /// Export format (json, csv, html, markdown, dot, assembly)
    pub format: Option<String>,

    /// Show detailed CFG analysis
    pub detailed: bool,

    /// Show only graph metrics
    pub metrics: bool,

    /// Show function analysis
    pub functions: bool,

    /// Show loop analysis
    pub loops: bool,

    pub files: Vec<PathBuf>,
}

impl Opts {
    pub fn parse() -> Self {
        let matches = Command::new("decompiler")
            .arg(Arg::new("raw").long("raw").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("cfg").long("cfg").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("tui").long("tui").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("output").long("output").short('o').value_name("FILE"))
            .arg(Arg::new("format").long("format").short('f').value_name("FORMAT"))
            .arg(Arg::new("detailed").long("detailed").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("metrics").long("metrics").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("functions").long("functions").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("loops").long("loops").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("files").value_name("FILE").num_args(1..))
            .get_matches();

        Self {
            raw: matches.get_flag("raw"),
            cfg: matches.get_flag("cfg"),
            tui: matches.get_flag("tui"),
            output: matches.get_one::<String>("output").map(PathBuf::from),
            format: matches.get_one::<String>("format").cloned(),
            detailed: matches.get_flag("detailed"),
            metrics: matches.get_flag("metrics"),
            functions: matches.get_flag("functions"),
            loops: matches.get_flag("loops"),
            files: matches.get_many::<String>("files")
                .unwrap_or_default()
                .map(PathBuf::from)
                .collect(),
        }
    }
}

fn main() -> io::Result<()> {
    let opts = Opts::parse();

    for file_path in &opts.files {
        // Only show loading messages if not launching TUI
        if !opts.tui {
            println!("[ANALYZING] {}", file_path.display());
            println!("{}", "=".repeat(60));
        }

        let data = {
            let mut f = BufReader::new(File::open(file_path)?);
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            buf
        };

        let TextSection { va, bytes } = parser::get_text_section(&data).unwrap_or_else(|e| {
            eprintln!("{}: failed to parse .text: {}", file_path.display(), e);
            std::process::exit(1);
        });

        if !opts.tui {
            println!("[OK] Loaded .text section: VA={:#x}, Size={} bytes", va, bytes.len());
        }

        let disasm_opts = DisasmOpts {
            base_address: va,
            bitness: 64,
        };

        let instructions = disasm(&bytes, disasm_opts);  // Simple call
        
        if !opts.tui {
            println!("[INFO] Disassembled {} instructions", instructions.len());
        }

        let cfg = if opts.cfg || opts.detailed || opts.metrics || opts.functions || opts.loops || opts.tui {
            // Convert to enhanced CFG format
            let cfg_instructions: Vec<CfgInstruction> = instructions.iter().map(|inst| {
                CfgInstruction {
                    address: Address(inst.address),
                    mnemonic: extract_mnemonic(&inst.text),
                    operands: extract_operands(&inst.text),
                    bytes: inst.bytes.clone(),
                }
            }).collect();

            let mut cfg = ControlFlowGraph::new();
            
            // Only show building message if not launching TUI
            if !opts.tui {
                println!("[BUILDING] Enhanced control flow graph...");
            }
            
            cfg.build_from_instructions(cfg_instructions);
            
            if !opts.tui {
                println!("[OK] CFG analysis complete!");
            }
            
            Some(cfg)
        } else {
            None
        };

        // Launch TUI immediately if requested, skip all other output
        if opts.tui {
            if let Err(e) = tui::run_tui(instructions, cfg) {
                eprintln!("[ERROR] TUI error: {}", e);
                std::process::exit(1);
            }
            continue; // Skip all other processing for this file when TUI is used
        }

        // Handle different display modes (only when NOT using TUI)
        match &cfg {
            Some(cfg) => {
                if opts.metrics {
                    display_metrics(cfg);
                } else if opts.functions {
                    display_functions(cfg);
                } else if opts.loops {
                    display_loops(cfg);
                } else if opts.detailed {
                    cfg.display_ascii();
                } else if opts.cfg {
                    cfg.display_ascii();
                }
            }
            None => {
                if opts.cfg {
                    println!("[ERROR] Use --cfg flag to enable control flow analysis");
                }
            }
        }

        // Handle export
        if let Some(output_path) = &opts.output {
            println!("\n[EXPORT] Exporting analysis...");
            let export_result = if let Some(format_str) = &opts.format {
                let format = parse_export_format(format_str);
                match &cfg {
                    Some(cfg) => Exporter::export_with_cfg(&instructions, cfg, format, output_path.to_str().unwrap()),
                    None => Exporter::export_instructions(&instructions, format, output_path.to_str().unwrap()),
                }
            } else {
                export_auto_format(&instructions, cfg.as_ref(), output_path.to_str().unwrap())
            };

            if let Err(e) = export_result {
                eprintln!("[ERROR] Export failed: {}", e);
                std::process::exit(1);
            }
        }

        // Default output (only when NOT using TUI)
        if !opts.cfg && !opts.detailed && !opts.metrics && !opts.functions && !opts.loops && opts.output.is_none() {
            println!("\n[DISASM] Basic Disassembly:");
            for (i, inst) in instructions.iter().enumerate() {
                if i < 20 { // Limit output for readability
                    println!("  {}", inst);
                } else if i == 20 {
                    println!("  ... ({} more instructions)", instructions.len() - 20);
                    break;
                }
            }
        }

        println!("\n{}\n", "=".repeat(60));
    }

    Ok(())
}

fn display_metrics(cfg: &ControlFlowGraph) {
    // Calculate basic metrics from the CFG
    let total_blocks = cfg.blocks.len();
    let total_edges = cfg.edges.len();
    
    // Fixed: Use saturating arithmetic
    let cyclomatic_complexity = total_edges.saturating_sub(total_blocks).saturating_add(2);
    
    let average_block_size = if total_blocks > 0 {
        cfg.blocks.values().map(|b| b.instructions.len()).sum::<usize>() as f64 / total_blocks as f64
    } else {
        0.0
    };
    let branching_factor = if total_blocks > 0 {
        total_edges as f64 / total_blocks as f64
    } else {
        0.0
    };
    
    println!("\n[METRICS] Graph Metrics:");
    println!("+-----------------------------+-------------+");
    println!("| Metric                      | Value       |");
    println!("+-----------------------------+-------------+");
    println!("| Total Blocks                | {:>11} |", total_blocks);
    println!("| Total Edges                 | {:>11} |", total_edges);
    println!("| Cyclomatic Complexity       | {:>11} |", cyclomatic_complexity);
    println!("| Average Block Size          | {:>11.2} |", average_block_size);
    println!("| Branching Factor            | {:>11.2} |", branching_factor);
    println!("+-----------------------------+-------------+");
    
    // Additional analysis
    println!("\n[ANALYSIS] Summary:");
    if cyclomatic_complexity > 10 {
        println!("  [WARNING] High complexity detected (CC: {})", cyclomatic_complexity);
    } else {
        println!("  [OK] Moderate complexity (CC: {})", cyclomatic_complexity);
    }
    
    if branching_factor > 2.0 {
        println!("  [INFO] High branching factor: {:.2}", branching_factor);
    }
    
    // Note: Dead code analysis would require additional implementation
    // if !cfg.unreachable_blocks.is_empty() {
    //     println!("  [WARNING] Dead code detected: {} unreachable blocks", cfg.unreachable_blocks.len());
    // }
}

fn display_functions(cfg: &ControlFlowGraph) {
    println!("\n[FUNCTIONS] Function Analysis:");
    println!("  Function analysis not yet implemented");
    println!("  Available basic blocks: {}", cfg.blocks.len());
    println!("  Available edges: {}", cfg.edges.len());
}

fn display_loops(cfg: &ControlFlowGraph) {
    println!("\n[LOOPS] Loop Analysis:");
    
    // Basic loop detection using back edges
    let mut back_edges = Vec::new();
    
    for edge in &cfg.edges {
        // A back edge is an edge from a block to a block that appears earlier in address order
        if edge.to.0 <= edge.from.0 {
            back_edges.push((&edge.from, &edge.to));
        }
    }
    
    if back_edges.is_empty() {
        println!("  No loops detected");
        return;
    }
    
    println!("  Detected {} potential loop(s):", back_edges.len());
    for (i, (from, to)) in back_edges.iter().enumerate() {
        println!("\n[LOOP] Loop {} (Simple back-edge detection)", i + 1);
        println!("+-- Back edge: {:#x} -> {:#x}", from.0, to.0);
        println!("+-- Loop header (estimated): {:#x}", to.0);
        println!("+-- Loop latch (estimated): {:#x}", from.0);
    }
}

fn extract_mnemonic(instruction_text: &str) -> String {
    instruction_text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

fn extract_operands(instruction_text: &str) -> String {
    let parts: Vec<&str> = instruction_text.splitn(2, ' ').collect();
    if parts.len() > 1 {
        parts[1].to_string()
    } else {
        String::new()
    }
}

fn parse_export_format(format_str: &str) -> ExportFormat {
    match format_str.to_lowercase().as_str() {
        "json" => ExportFormat::Json,
        "csv" => ExportFormat::Csv,
        "html" => ExportFormat::Html,
        "markdown" | "md" => ExportFormat::Markdown,
        "dot" => ExportFormat::Dot,
        "assembly" | "asm" => ExportFormat::Assembly,
        _ => {
            eprintln!("Unknown format '{}', defaulting to JSON", format_str);
            ExportFormat::Json
        }
    }
}

