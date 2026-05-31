use clap::{Arg, Command};
use disassembler::arch::arm::ARMDisassembler;
use disassembler::arch::x86::{disasm, DisasmOpts};
use disassembler::arch::{ArchConfig, ArchDisassembler, Architecture};
use disassembler::export::{export_auto_format_with_metadata_and_analysis, ExportFormat, Exporter};
use disassembler::graph::{Address, ControlFlowGraph, Instruction as CfgInstruction};
use disassembler::parser::{self, AnalyzedBinary, BinaryAnalysis, BinaryMetadata};
use disassembler::project::AnalysisProject;
use disassembler::tui;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Opts {
    pub raw: bool,
    pub cfg: bool,
    pub tui: bool,

    pub output: Option<PathBuf>,

    pub project: Option<PathBuf>,

    pub save_project: Option<PathBuf>,

    pub format: Option<String>,

    pub detailed: bool,

    pub metrics: bool,

    pub functions: bool,

    pub names: bool,

    pub loops: bool,

    pub files: Vec<PathBuf>,
}

impl Opts {
    pub fn parse() -> Self {
        let matches = Command::new("disassembler")
            .arg(Arg::new("raw").long("raw").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("cfg").long("cfg").action(clap::ArgAction::SetTrue))
            .arg(Arg::new("tui").long("tui").action(clap::ArgAction::SetTrue))
            .arg(
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .value_name("FILE"),
            )
            .arg(Arg::new("project").long("project").value_name("FILE"))
            .arg(
                Arg::new("save-project")
                    .long("save-project")
                    .value_name("FILE"),
            )
            .arg(
                Arg::new("format")
                    .long("format")
                    .short('f')
                    .value_name("FORMAT"),
            )
            .arg(
                Arg::new("detailed")
                    .long("detailed")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("metrics")
                    .long("metrics")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("functions")
                    .long("functions")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("names")
                    .long("names")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("loops")
                    .long("loops")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(Arg::new("files").value_name("FILE").num_args(1..))
            .get_matches();

        Self {
            raw: matches.get_flag("raw"),
            cfg: matches.get_flag("cfg"),
            tui: matches.get_flag("tui"),
            output: matches.get_one::<String>("output").map(PathBuf::from),
            project: matches.get_one::<String>("project").map(PathBuf::from),
            save_project: matches.get_one::<String>("save-project").map(PathBuf::from),
            format: matches.get_one::<String>("format").cloned(),
            detailed: matches.get_flag("detailed"),
            metrics: matches.get_flag("metrics"),
            functions: matches.get_flag("functions"),
            names: matches.get_flag("names"),
            loops: matches.get_flag("loops"),
            files: matches
                .get_many::<String>("files")
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

        let AnalyzedBinary {
            metadata,
            text,
            analysis,
        } = parser::analyze_binary(&data).unwrap_or_else(|e| {
            eprintln!("{}: failed to analyze binary: {}", file_path.display(), e);
            std::process::exit(1);
        });

        let loaded_project = opts.project.as_ref().map(|project_path| {
            AnalysisProject::load(project_path).unwrap_or_else(|e| {
                eprintln!(
                    "{}: failed to load project {}: {}",
                    file_path.display(),
                    project_path.display(),
                    e
                );
                std::process::exit(1);
            })
        });

        if let Some(project) = &loaded_project {
            if !opts.tui {
                println!(
                    "[PROJECT] Loaded {}: {} names, {} comments, {} bookmarks",
                    opts.project.as_ref().unwrap().display(),
                    project.user_names.len(),
                    project.comments.len(),
                    project.bookmarks.len()
                );
            }
        }

        if !opts.tui {
            println!(
                "[OK] Loaded {} .text section: VA={:#x}, Size={} bytes, Arch={}, Endian={}",
                metadata.format,
                text.va,
                text.bytes.len(),
                metadata.architecture,
                metadata.endianness
            );
        }

        let instructions = disassemble_text(text.bytes, text.va, &metadata).unwrap_or_else(|e| {
            eprintln!("{}: failed to disassemble: {}", file_path.display(), e);
            std::process::exit(1);
        });

        if !opts.tui {
            println!("[INFO] Disassembled {} instructions", instructions.len());
        }

        let cfg = if opts.cfg
            || opts.detailed
            || opts.metrics
            || opts.functions
            || opts.loops
            || opts.tui
        {
            // Convert to enhanced CFG format
            let cfg_instructions: Vec<CfgInstruction> = instructions
                .iter()
                .map(|inst| CfgInstruction {
                    address: Address(inst.address),
                    mnemonic: extract_mnemonic(&inst.text),
                    operands: extract_operands(&inst.text),
                    bytes: inst.bytes.clone(),
                })
                .collect();

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
            if let Err(e) = tui::run_tui(instructions, cfg, analysis.clone()) {
                eprintln!("[ERROR] TUI error: {}", e);
                std::process::exit(1);
            }
            return Ok(());
        }

        // Handle different display modes (only when NOT using TUI)
        match &cfg {
            Some(cfg) => {
                if opts.metrics {
                    display_metrics(cfg);
                } else if opts.functions {
                    display_functions(cfg);
                } else if opts.names {
                    display_names(&analysis);
                } else if opts.loops {
                    display_loops(cfg);
                } else if opts.detailed || opts.cfg {
                    cfg.display_ascii();
                }
            }
            None => {
                if opts.names {
                    display_names(&analysis);
                } else if opts.cfg {
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
                    Some(cfg) => Exporter::export_with_cfg_metadata_and_analysis(
                        &instructions,
                        cfg,
                        format,
                        output_path.to_str().unwrap(),
                        Some(&metadata),
                        Some(&analysis),
                    ),
                    None => Exporter::export_instructions_with_metadata_and_analysis(
                        &instructions,
                        format,
                        output_path.to_str().unwrap(),
                        Some(&metadata),
                        Some(&analysis),
                    ),
                }
            } else {
                export_auto_format_with_metadata_and_analysis(
                    &instructions,
                    cfg.as_ref(),
                    output_path.to_str().unwrap(),
                    Some(&metadata),
                    Some(&analysis),
                )
            };

            if let Err(e) = export_result {
                eprintln!("[ERROR] Export failed: {}", e);
                std::process::exit(1);
            }
        }

        if let Some(project_path) = &opts.save_project {
            let mut project = loaded_project
                .clone()
                .unwrap_or_else(|| AnalysisProject::from_binary(file_path, &data));
            if project.functions.is_empty() {
                if let Some(cfg) = &cfg {
                    project.functions = cfg
                        .function_summaries()
                        .into_iter()
                        .map(|function| disassembler::project::ProjectFunction {
                            entry: function.entry.0,
                            name: None,
                        })
                        .collect();
                }
            }
            project.save(project_path)?;
            if !opts.tui {
                println!(
                    "[PROJECT] Saved {}: {} names, {} comments, {} bookmarks, {} functions",
                    project_path.display(),
                    project.user_names.len(),
                    project.comments.len(),
                    project.bookmarks.len(),
                    project.functions.len()
                );
            }
        }

        // Default output (only when NOT using TUI)
        if !opts.cfg
            && !opts.detailed
            && !opts.metrics
            && !opts.functions
            && !opts.names
            && !opts.loops
            && opts.output.is_none()
        {
            println!("\n[DISASM] Basic Disassembly:");
            for (i, inst) in instructions.iter().enumerate() {
                if i < 20 {
                    // Limit output for readability
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

fn display_names(analysis: &BinaryAnalysis) {
    println!("\n[NAMES] Binary Names:");

    println!(
        "  Imports: {}  Symbols: {}  Strings: {}",
        analysis.imports.len(),
        analysis.symbols.len(),
        analysis.strings.len()
    );

    println!("\n[IMPORTS]");
    if analysis.imports.is_empty() {
        println!("  No imports found");
    } else {
        for import in analysis.imports.iter().take(30) {
            let address = import
                .address
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "unknown".to_string());
            let library = import.library.as_deref().unwrap_or("unknown");
            println!("  {:>14}  {:<24} {}", address, library, import.name);
        }
        if analysis.imports.len() > 30 {
            println!("  ... {} more imports", analysis.imports.len() - 30);
        }
    }

    println!("\n[SYMBOLS]");
    if analysis.symbols.is_empty() {
        println!("  No symbols found");
    } else {
        for symbol in analysis.symbols.iter().take(30) {
            let address = symbol
                .address
                .map(|address| format!("{address:#x}"))
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "  {:>14}  {:<8} {}",
                address,
                symbol.kind.as_str(),
                symbol.name
            );
        }
        if analysis.symbols.len() > 30 {
            println!("  ... {} more symbols", analysis.symbols.len() - 30);
        }
    }

    println!("\n[STRINGS]");
    if analysis.strings.is_empty() {
        println!("  No printable strings found");
    } else {
        for string in analysis.strings.iter().take(30) {
            println!(
                "  {:#014x}  {}",
                string.address,
                truncate_for_display(&string.value, 96)
            );
        }
        if analysis.strings.len() > 30 {
            println!("  ... {} more strings", analysis.strings.len() - 30);
        }
    }
}

fn truncate_for_display(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        truncated.push_str("...");
    }
    truncated
}

fn disassemble_text(
    bytes: &[u8],
    base_address: u64,
    metadata: &BinaryMetadata,
) -> Result<Vec<disassembler::Instruction>, Box<dyn std::error::Error>> {
    match metadata.architecture {
        Architecture::X86 | Architecture::X64 => Ok(disasm(
            bytes,
            DisasmOpts {
                base_address,
                bitness: metadata.bitness,
            },
        )),
        Architecture::ARM | Architecture::AArch64 => {
            let disassembler = ARMDisassembler;
            let config = ArchConfig {
                arch: metadata.architecture,
                bitness: metadata.bitness,
                endianness: metadata.endianness,
                base_address,
            };
            Ok(disassembler.disassemble(bytes, &config))
        }
        unsupported => Err(format!("{} disassembly is not implemented yet", unsupported).into()),
    }
}

fn display_metrics(cfg: &ControlFlowGraph) {
    // Calculate basic metrics from the CFG
    let total_blocks = cfg.blocks.len();
    let total_edges = cfg.edges.len();

    // Fixed: Use saturating arithmetic
    let cyclomatic_complexity = total_edges.saturating_sub(total_blocks).saturating_add(2);

    let average_block_size = if total_blocks > 0 {
        cfg.blocks
            .values()
            .map(|b| b.instructions.len())
            .sum::<usize>() as f64
            / total_blocks as f64
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
    println!(
        "| Cyclomatic Complexity       | {:>11} |",
        cyclomatic_complexity
    );
    println!(
        "| Average Block Size          | {:>11.2} |",
        average_block_size
    );
    println!(
        "| Branching Factor            | {:>11.2} |",
        branching_factor
    );
    println!("+-----------------------------+-------------+");

    // Additional analysis
    println!("\n[ANALYSIS] Summary:");
    if cyclomatic_complexity > 10 {
        println!(
            "  [WARNING] High complexity detected (CC: {})",
            cyclomatic_complexity
        );
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
    let functions = cfg.function_summaries();

    if functions.is_empty() {
        println!("  No function entries inferred");
        return;
    }

    println!("+----------------+--------+--------------+-------+---------+");
    println!("| Entry          | Blocks | Instructions | Edges | Callers |");
    println!("+----------------+--------+--------------+-------+---------+");
    for function in functions {
        println!(
            "| {:#014x} | {:>6} | {:>12} | {:>5} | {:>7} |",
            function.entry.0,
            function.block_count,
            function.instruction_count,
            function.edge_count,
            function.caller_count
        );
    }
    println!("+----------------+--------+--------------+-------+---------+");
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
