use clap::{Arg, Command};
use disassembler::arch::arm::ARMDisassembler;
use disassembler::arch::x86::{disasm, DisasmOpts};
use disassembler::arch::{ArchConfig, ArchDisassembler, Architecture};
use disassembler::export::{export_auto_format_with_metadata_and_analysis, ExportFormat, Exporter};
use disassembler::graph::{
    is_known_noreturn_symbol, Address, ControlFlowGraph, ExternalCallTarget, FunctionSeed,
    Instruction as CfgInstruction, NoreturnCallTarget,
};
use disassembler::parser::{
    self, AnalyzedBinary, BinaryAnalysis, BinaryMetadata, LoadedSection, TextSection,
};
use disassembler::project::{AnalysisProject, ProjectFunction};
use disassembler::tui;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Opts {
    pub raw: bool,
    pub cfg: bool,
    pub tui: bool,

    pub overview: bool,

    pub output: Option<PathBuf>,

    pub project: Option<PathBuf>,

    pub save_project: Option<PathBuf>,

    pub format: Option<String>,

    pub detailed: bool,

    pub metrics: bool,

    pub functions: bool,

    pub callgraph: bool,

    pub names: bool,

    pub imports: bool,

    pub exports: bool,

    pub symbols: bool,

    pub sections: bool,

    pub data: bool,

    pub relocations: bool,

    pub unwind: bool,

    pub all_executable: bool,

    pub section: Option<String>,

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
                Arg::new("overview")
                    .long("overview")
                    .help("Show binary orientation, metadata, section mix, and analysis counts")
                    .action(clap::ArgAction::SetTrue),
            )
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
                Arg::new("callgraph")
                    .long("callgraph")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("names")
                    .long("names")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("imports")
                    .long("imports")
                    .help("Show imported libraries and import address table entries")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("exports")
                    .long("exports")
                    .help("Show exported symbols and forwarders")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("symbols")
                    .long("symbols")
                    .help("Show parsed binary symbols")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("sections")
                    .long("sections")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("data")
                    .long("data")
                    .help("Show decoded data-section pointer objects")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("relocations")
                    .long("relocations")
                    .help("Show loader relocation entries")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("unwind")
                    .long("unwind")
                    .help("Show exception/unwind-backed function ranges")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("all-executable")
                    .long("all-executable")
                    .help("Disassemble all file-backed executable sections instead of only .text")
                    .action(clap::ArgAction::SetTrue),
            )
            .arg(
                Arg::new("section")
                    .long("section")
                    .value_name("NAME_OR_VA")
                    .help("Disassemble a single executable section by name or virtual address"),
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
            overview: matches.get_flag("overview"),
            output: matches.get_one::<String>("output").map(PathBuf::from),
            project: matches.get_one::<String>("project").map(PathBuf::from),
            save_project: matches.get_one::<String>("save-project").map(PathBuf::from),
            format: matches.get_one::<String>("format").cloned(),
            detailed: matches.get_flag("detailed"),
            metrics: matches.get_flag("metrics"),
            functions: matches.get_flag("functions"),
            callgraph: matches.get_flag("callgraph"),
            names: matches.get_flag("names"),
            imports: matches.get_flag("imports"),
            exports: matches.get_flag("exports"),
            symbols: matches.get_flag("symbols"),
            sections: matches.get_flag("sections"),
            data: matches.get_flag("data"),
            relocations: matches.get_flag("relocations"),
            unwind: matches.get_flag("unwind"),
            all_executable: matches.get_flag("all-executable"),
            section: matches.get_one::<String>("section").cloned(),
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

        let disassembly_sections = select_disassembly_sections(&data, &analysis, &text, &opts)
            .unwrap_or_else(|e| {
                eprintln!(
                    "{}: failed to select code sections: {}",
                    file_path.display(),
                    e
                );
                std::process::exit(1);
            });

        if !opts.tui {
            let section_names = disassembly_sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let byte_count: usize = disassembly_sections
                .iter()
                .map(|section| section.bytes.len())
                .sum();
            println!(
                "[OK] Loaded {} code: {} section(s), {} bytes [{}], Arch={}, Endian={}",
                metadata.format,
                disassembly_sections.len(),
                byte_count,
                section_names,
                metadata.architecture,
                metadata.endianness
            );
        }

        let instructions =
            disassemble_sections(&disassembly_sections, &metadata).unwrap_or_else(|e| {
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
            || opts.callgraph
            || opts.overview
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

            cfg.build_from_instructions_with_function_seeds_and_noreturn_targets(
                cfg_instructions,
                build_cfg_function_seeds(&metadata, &analysis),
                build_noreturn_call_targets(&analysis),
            );

            if !opts.tui {
                println!("[OK] CFG analysis complete!");
            }

            Some(cfg)
        } else {
            None
        };

        // Launch TUI immediately if requested, skip all other output
        if opts.tui {
            let mut project = loaded_project
                .clone()
                .unwrap_or_else(|| AnalysisProject::from_binary(file_path, &data));
            populate_project_functions(&mut project, cfg.as_ref());
            let save_project = opts.save_project.clone().or_else(|| opts.project.clone());

            if let Err(e) = tui::run_tui_with_project_binary_and_metadata(
                instructions,
                data.clone(),
                Some(metadata.clone()),
                cfg,
                analysis.clone(),
                Some(project),
                save_project,
            ) {
                eprintln!("[ERROR] TUI error: {}", e);
                std::process::exit(1);
            }
            return Ok(());
        }

        // Handle different display modes (only when NOT using TUI)
        match &cfg {
            Some(cfg) => {
                if opts.overview {
                    display_overview(
                        file_path,
                        data.len(),
                        &metadata,
                        &analysis,
                        instructions.len(),
                        Some(cfg),
                        loaded_project.as_ref(),
                    );
                } else if opts.metrics {
                    display_metrics(cfg);
                } else if opts.functions {
                    display_functions(cfg);
                } else if opts.callgraph {
                    display_call_graph(cfg, &analysis);
                } else if opts.names {
                    display_names(&analysis);
                } else if opts.imports {
                    display_imports(&analysis);
                } else if opts.exports {
                    display_exports(&analysis);
                } else if opts.symbols {
                    display_symbols(&analysis);
                } else if opts.sections {
                    display_sections(&analysis);
                } else if opts.data {
                    display_data_objects(&analysis);
                } else if opts.relocations {
                    display_relocations(&analysis);
                } else if opts.unwind {
                    display_unwind_functions(&analysis);
                } else if opts.loops {
                    display_loops(cfg);
                } else if opts.detailed || opts.cfg {
                    cfg.display_ascii();
                }
            }
            None => {
                if opts.overview {
                    display_overview(
                        file_path,
                        data.len(),
                        &metadata,
                        &analysis,
                        instructions.len(),
                        None,
                        loaded_project.as_ref(),
                    );
                } else if opts.names {
                    display_names(&analysis);
                } else if opts.imports {
                    display_imports(&analysis);
                } else if opts.exports {
                    display_exports(&analysis);
                } else if opts.symbols {
                    display_symbols(&analysis);
                } else if opts.sections {
                    display_sections(&analysis);
                } else if opts.data {
                    display_data_objects(&analysis);
                } else if opts.relocations {
                    display_relocations(&analysis);
                } else if opts.unwind {
                    display_unwind_functions(&analysis);
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
            populate_project_functions(&mut project, cfg.as_ref());
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
            && !opts.overview
            && !opts.functions
            && !opts.callgraph
            && !opts.names
            && !opts.imports
            && !opts.exports
            && !opts.symbols
            && !opts.sections
            && !opts.data
            && !opts.relocations
            && !opts.unwind
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

fn populate_project_functions(project: &mut AnalysisProject, cfg: Option<&ControlFlowGraph>) {
    if !project.functions.is_empty() {
        return;
    }

    if let Some(cfg) = cfg {
        project.functions = cfg
            .function_summaries()
            .into_iter()
            .map(|function| ProjectFunction {
                entry: function.entry.0,
                name: None,
            })
            .collect();
    }
}

fn build_external_call_targets(analysis: &BinaryAnalysis) -> Vec<ExternalCallTarget> {
    analysis
        .imports
        .iter()
        .filter_map(|import| {
            let address = import.address?;
            let library = import.library.as_deref().unwrap_or("unknown");
            Some(ExternalCallTarget {
                address: Address(address),
                label: format!("{library}!{}", import.name),
            })
        })
        .collect()
}

fn build_noreturn_call_targets(analysis: &BinaryAnalysis) -> Vec<NoreturnCallTarget> {
    analysis
        .imports
        .iter()
        .filter(|import| is_known_noreturn_symbol(&import.name))
        .filter_map(|import| {
            let address = import.address?;
            let library = import.library.as_deref().unwrap_or("unknown");
            Some(NoreturnCallTarget {
                address: Address(address),
                label: format!("{library}!{}", import.name),
            })
        })
        .collect()
}

fn build_cfg_function_seeds(
    metadata: &BinaryMetadata,
    analysis: &BinaryAnalysis,
) -> Vec<FunctionSeed> {
    let mut seeds = Vec::new();

    if let Some(entry_point) = metadata.entry_point {
        if is_executable_function_seed_address(analysis, entry_point) {
            seeds.push(FunctionSeed::entry_point(Address(entry_point)));
        }
    }

    seeds.extend(analysis.symbols.iter().filter_map(|symbol| {
        if !matches!(&symbol.kind, parser::SymbolKind::Function) {
            return None;
        }

        let address = symbol.address?;
        is_executable_function_seed_address(analysis, address)
            .then(|| FunctionSeed::symbol(Address(address)))
    }));

    seeds.extend(analysis.exports.iter().filter_map(|export| {
        if export.forwarder.is_some() {
            return None;
        }

        let address = export.address?;
        is_executable_function_seed_address(analysis, address)
            .then(|| FunctionSeed::export(Address(address)))
    }));

    seeds.extend(
        analysis
            .function_ranges
            .iter()
            .filter(|function| is_executable_function_seed_address(analysis, function.start))
            .map(|function| FunctionSeed::unwind(Address(function.start))),
    );

    seeds
}

fn is_executable_function_seed_address(analysis: &BinaryAnalysis, address: u64) -> bool {
    analysis
        .section_containing_address(address)
        .is_some_and(|section| {
            section.is_executable_code_candidate() && address < section.file_backed_end_address()
        })
}

fn display_overview(
    file_path: &std::path::Path,
    file_size: usize,
    metadata: &BinaryMetadata,
    analysis: &BinaryAnalysis,
    instruction_count: usize,
    cfg: Option<&ControlFlowGraph>,
    project: Option<&AnalysisProject>,
) {
    println!("\n[OVERVIEW] Binary Orientation:");
    println!("  Path: {}", file_path.display());
    println!("  Size: {} bytes", file_size);
    println!("  Format: {}", metadata.format);
    println!(
        "  Architecture: {} / {}-bit / {} endian",
        metadata.architecture, metadata.bitness, metadata.endianness
    );
    println!(
        "  Entry Point: {}",
        metadata
            .entry_point
            .map(|entry| format!("{entry:#x}"))
            .unwrap_or_else(|| "unknown".to_string())
    );

    if let Some(project) = project {
        println!("  Fingerprint: {}", project.binary.fingerprint);
        println!(
            "  Project: {} names, {} comments, {} bookmarks, {} functions",
            project.user_names.len(),
            project.comments.len(),
            project.bookmarks.len(),
            project.functions.len()
        );
    }

    println!("\n[ANALYSIS]");
    println!("  Instructions: {instruction_count}");
    let call_graph =
        cfg.map(|cfg| cfg.call_graph_with_external_targets(build_external_call_targets(analysis)));
    println!(
        "  Functions: {}  Call edges: {}  Import thunks: {}  Noreturn imports: {}",
        call_graph.as_ref().map_or(0, |graph| graph.functions.len()),
        call_graph
            .as_ref()
            .map_or(0, |graph| graph.total_edge_count()),
        call_graph.as_ref().map_or(0, |graph| graph
            .functions
            .iter()
            .filter(|function| function.import_thunk.is_some())
            .count()),
        cfg.map_or(0, ControlFlowGraph::noreturn_call_target_count)
    );
    println!(
        "  Imports: {}  Exports: {}  Symbols: {}  Strings: {}  Data pointers: {}  Relocations: {}  Unwind funcs: {}",
        analysis.imports.len(),
        analysis.exports.len(),
        analysis.symbols.len(),
        analysis.strings.len(),
        analysis.data_objects.len(),
        analysis.relocations.len(),
        analysis.function_ranges.len()
    );
    println!("  Sections indexed: {}", analysis.sections.len());

    let executable = analysis
        .sections
        .iter()
        .filter(|section| section.executable)
        .count();
    let data_like = analysis
        .sections
        .iter()
        .filter(|section| section.is_string_candidate())
        .count();
    let writable = analysis
        .sections
        .iter()
        .filter(|section| section.writable)
        .count();
    println!("\n[SECTIONS]");
    println!(
        "  Total: {}  Executable: {}  Data-like: {}  Writable: {}",
        analysis.sections.len(),
        executable,
        data_like,
        writable
    );

    let mut largest = analysis.sections.clone();
    largest.sort_by_key(|section| std::cmp::Reverse(section.file_size.max(section.virtual_size)));
    for section in largest.iter().take(8) {
        println!(
            "  {:<14} {}  va={:#x}  virt={:#x}  file={:#x}",
            truncate_for_display(&section.name, 14),
            section.permissions(),
            section.address,
            section.virtual_size,
            section.file_size
        );
    }
}
fn display_imports(analysis: &BinaryAnalysis) {
    println!("\n[IMPORTS] Imported Libraries and IAT Entries:");

    if analysis.imports.is_empty() {
        println!("  No imports found");
        return;
    }

    let library_count = analysis
        .imports
        .iter()
        .filter_map(|import| import.library.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    println!(
        "  Imports: {} entries across {} libraries",
        analysis.imports.len(),
        library_count
    );
    println!("+----------------+--------------------------+--------------------------------+");
    println!("| IAT Address    | Library                  | Name                           |");
    println!("+----------------+--------------------------+--------------------------------+");
    for import in analysis.imports.iter().take(120) {
        let address = import
            .address
            .map(|address| format!("{address:#014x}"))
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "| {:<14} | {:<24} | {:<30} |",
            address,
            truncate_for_display(import.library.as_deref().unwrap_or("unknown"), 24),
            truncate_for_display(&import.name, 30)
        );
    }
    println!("+----------------+--------------------------+--------------------------------+");
    if analysis.imports.len() > 120 {
        println!("  ... {} more imports", analysis.imports.len() - 120);
    }
}
fn display_exports(analysis: &BinaryAnalysis) {
    println!("\n[EXPORTS] Exported Symbols:");

    if analysis.exports.is_empty() {
        println!("  No exports found");
        return;
    }

    println!("  Exports: {} entries", analysis.exports.len());
    println!("+----------------+----------+--------+--------------------------------+------------------------------+");
    println!("| Address        | Kind     | Size   | Name                           | Forwarder                    |");
    println!("+----------------+----------+--------+--------------------------------+------------------------------+");
    for export in analysis.exports.iter().take(120) {
        let address = export
            .address
            .map(|address| format!("{address:#014x}"))
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "| {:<14} | {:<8} | {:>6} | {:<30} | {:<28} |",
            address,
            export.kind.as_str(),
            export.size,
            truncate_for_display(&export.name, 30),
            truncate_for_display(export.forwarder.as_deref().unwrap_or(""), 28)
        );
    }
    println!("+----------------+----------+--------+--------------------------------+------------------------------+");
    if analysis.exports.len() > 120 {
        println!("  ... {} more exports", analysis.exports.len() - 120);
    }
}

fn display_symbols(analysis: &BinaryAnalysis) {
    println!("\n[SYMBOLS] Binary Symbols:");

    if analysis.symbols.is_empty() {
        println!("  No symbols found");
        return;
    }

    println!("  Symbols: {} entries", analysis.symbols.len());
    println!("+----------------+----------+--------------------------------+");
    println!("| Address        | Kind     | Name                           |");
    println!("+----------------+----------+--------------------------------+");
    for symbol in analysis.symbols.iter().take(120) {
        let address = symbol
            .address
            .map(|address| format!("{address:#014x}"))
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "| {:<14} | {:<8} | {:<30} |",
            address,
            symbol.kind.as_str(),
            truncate_for_display(&symbol.name, 30)
        );
    }
    println!("+----------------+----------+--------------------------------+");
    if analysis.symbols.len() > 120 {
        println!("  ... {} more symbols", analysis.symbols.len() - 120);
    }
}
fn display_relocations(analysis: &BinaryAnalysis) {
    println!("\n[RELOCATIONS] Loader Relocations:");

    if analysis.relocations.is_empty() {
        println!("  No relocations found");
        return;
    }

    println!("  Relocations: {} entries", analysis.relocations.len());
    println!("+----------------+--------------+--------------+--------------+------------------------------+------------+");
    println!("| Address        | Section      | Source       | Type         | Symbol                       | Addend     |");
    println!("+----------------+--------------+--------------+--------------+------------------------------+------------+");
    for relocation in analysis.relocations.iter().take(120) {
        let addend = relocation
            .addend
            .map(|addend| format!("{addend:#x}"))
            .unwrap_or_default();
        println!(
            "| {:#014x} | {:<12} | {:<12} | {:<12} | {:<28} | {:>10} |",
            relocation.address,
            truncate_for_display(relocation.section.as_deref().unwrap_or("unknown"), 12),
            truncate_for_display(&relocation.source, 12),
            truncate_for_display(&relocation.kind, 12),
            truncate_for_display(relocation.symbol.as_deref().unwrap_or(""), 28),
            addend
        );
    }
    println!("+----------------+--------------+--------------+--------------+------------------------------+------------+");
    if analysis.relocations.len() > 120 {
        println!(
            "  ... {} more relocations",
            analysis.relocations.len() - 120
        );
    }
}

fn display_unwind_functions(analysis: &BinaryAnalysis) {
    println!("\n[UNWIND] Exception/Unwind Function Ranges:");

    if analysis.function_ranges.is_empty() {
        println!("  No unwind-backed function ranges found");
        return;
    }

    println!(
        "  Function ranges: {} entries",
        analysis.function_ranges.len()
    );
    println!("+----------------+----------------+----------+--------------+--------------+----------------+");
    println!("| Start          | End            | Size     | Section      | Source       | Unwind Info    |");
    println!("+----------------+----------------+----------+--------------+--------------+----------------+");
    for function in analysis.function_ranges.iter().take(160) {
        let unwind_info = function
            .unwind_info
            .map(|address| format!("{address:#x}"))
            .unwrap_or_default();
        println!(
            "| {:#014x} | {:#014x} | {:>8x} | {:<12} | {:<12} | {:<14} |",
            function.start,
            function.end,
            function.end.saturating_sub(function.start),
            truncate_for_display(function.section.as_deref().unwrap_or("unknown"), 12),
            truncate_for_display(&function.source, 12),
            truncate_for_display(&unwind_info, 14)
        );
    }
    println!("+----------------+----------------+----------+--------------+--------------+----------------+");
    if analysis.function_ranges.len() > 160 {
        println!(
            "  ... {} more function ranges",
            analysis.function_ranges.len() - 160
        );
    }
}

fn display_sections(analysis: &BinaryAnalysis) {
    println!("\n[SECTIONS] Binary Sections:");

    if analysis.sections.is_empty() {
        println!("  No sections found");
        return;
    }

    println!("+----------------+----------------+----------+----------+------+----------------+");
    println!("| Name           | VA             | VirtSize | FileSize | Perm | File Offset    |");
    println!("+----------------+----------------+----------+----------+------+----------------+");
    for section in &analysis.sections {
        println!(
            "| {:<14} | {:#014x} | {:>8x} | {:>8x} | {:<4} | {:#014x} |",
            truncate_for_display(&section.name, 14),
            section.address,
            section.virtual_size,
            section.file_size,
            section.permissions(),
            section.file_offset
        );
    }
    println!("+----------------+----------------+----------+----------+------+----------------+");
}

fn display_data_objects(analysis: &BinaryAnalysis) {
    println!("\n[DATA] Data Objects:");

    if analysis.data_objects.is_empty() {
        println!("  No data pointers found");
        return;
    }

    println!("+----------------+--------------+----------+----------------+----------------+------------------------------+");
    println!("| Address        | Section      | Kind     | Value          | Target         | Label                        |");
    println!("+----------------+--------------+----------+----------------+----------------+------------------------------+");
    for object in analysis.data_objects.iter().take(80) {
        println!(
            "| {:#014x} | {:<12} | {:<8} | {:#014x} | {:#014x} | {:<28} |",
            object.address,
            truncate_for_display(object.section.as_deref().unwrap_or("unknown"), 12),
            object.kind.as_str(),
            object.value,
            object.target,
            truncate_for_display(
                object
                    .target_label
                    .as_deref()
                    .or(object.target_section.as_deref())
                    .unwrap_or("unknown"),
                28
            )
        );
    }
    println!("+----------------+--------------+----------+----------------+----------------+------------------------------+");
    if analysis.data_objects.len() > 80 {
        println!(
            "  ... {} more data objects",
            analysis.data_objects.len() - 80
        );
    }
}

fn display_names(analysis: &BinaryAnalysis) {
    println!("\n[NAMES] Binary Names:");

    println!(
        "  Imports: {}  Exports: {}  Symbols: {}  Strings: {}",
        analysis.imports.len(),
        analysis.exports.len(),
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
                "  {:#014x}  {:<12}  {}",
                string.address,
                string.section.as_deref().unwrap_or("unknown"),
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

fn select_disassembly_sections<'a>(
    data: &'a [u8],
    analysis: &BinaryAnalysis,
    text: &TextSection<'a>,
    opts: &Opts,
) -> Result<Vec<LoadedSection<'a>>, Box<dyn std::error::Error>> {
    if let Some(selector) = opts.section.as_deref() {
        let Some(section) = find_section_selector(analysis, selector) else {
            return Err(format!("section not found: {selector}").into());
        };
        if !section.is_executable_code_candidate() {
            return Err(format!(
                "section {} is not a file-backed executable section",
                section.name
            )
            .into());
        }
        return Ok(vec![parser::load_section(data, section)?]);
    }

    if opts.all_executable || opts.tui {
        let sections = parser::executable_sections(data, analysis);
        if !sections.is_empty() {
            return Ok(sections);
        }
    }

    Ok(vec![LoadedSection {
        name: analysis
            .section_containing_address(text.va)
            .map(|section| section.name.clone())
            .unwrap_or_else(|| ".text".to_string()),
        va: text.va,
        bytes: text.bytes,
    }])
}

fn find_section_selector<'a>(
    analysis: &'a BinaryAnalysis,
    selector: &str,
) -> Option<&'a disassembler::parser::SectionSummary> {
    let selector = selector.trim();
    if let Some(address) = parse_section_address_selector(selector) {
        return analysis
            .sections
            .iter()
            .find(|section| section.address == address || section.contains_address(address));
    }

    analysis
        .sections
        .iter()
        .find(|section| section.name == selector || section.name.eq_ignore_ascii_case(selector))
}

fn parse_section_address_selector(selector: &str) -> Option<u64> {
    let selector = selector.trim();
    if selector.is_empty() {
        return None;
    }

    if let Some(hex) = selector
        .strip_prefix("0x")
        .or_else(|| selector.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok();
    }

    selector
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| selector.parse().ok())?
}

fn disassemble_sections(
    sections: &[LoadedSection<'_>],
    metadata: &BinaryMetadata,
) -> Result<Vec<disassembler::Instruction>, Box<dyn std::error::Error>> {
    let mut instructions = Vec::new();
    for section in sections {
        instructions.extend(disassemble_text(section.bytes, section.va, metadata)?);
    }
    instructions.sort_by_key(|instruction| instruction.address);
    Ok(instructions)
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

    println!("+----------------+----------+--------+--------+--------------+-------+---------+");
    println!("| Entry          | Kind     | Conf   | Blocks | Instructions | Edges | Callers |");
    println!("+----------------+----------+--------+--------+--------------+-------+---------+");
    for function in functions {
        println!(
            "| {:#014x} | {:<8} | {:<6} | {:>6} | {:>12} | {:>5} | {:>7} |",
            function.entry.0,
            function.kind.as_str(),
            function.confidence.as_str(),
            function.block_count,
            function.instruction_count,
            function.edge_count,
            function.caller_count
        );
    }
    println!("+----------------+----------+--------+--------+--------------+-------+---------+");
}

fn display_call_graph(cfg: &ControlFlowGraph, analysis: &BinaryAnalysis) {
    println!("\n[CALLGRAPH] Function Call Graph:");
    let call_graph = cfg.call_graph_with_external_targets(build_external_call_targets(analysis));
    let import_thunk_count = call_graph
        .functions
        .iter()
        .filter(|function| function.import_thunk.is_some())
        .count();

    if call_graph.functions.is_empty() {
        println!("  No function entries inferred");
        return;
    }

    println!(
        "  Functions: {}  Internal call edges: {}  External import edges: {}  Total edges: {}  Import thunks: {}",
        call_graph.functions.len(),
        call_graph.edges.len(),
        call_graph.external_edges.len(),
        call_graph.total_edge_count(),
        import_thunk_count
    );
    println!(
        "+----------------+----------+--------+----------+----------+--------+--------------+"
    );
    println!(
        "| Function       | Kind     | Conf   | Callers  | Calls    | Blocks | Instructions |"
    );
    println!(
        "+----------------+----------+--------+----------+----------+--------+--------------+"
    );
    for function in &call_graph.functions {
        println!(
            "| {:#014x} | {:<8} | {:<6} | {:>8} | {:>8} | {:>6} | {:>12} |",
            function.summary.entry.0,
            function.summary.kind.as_str(),
            function.summary.confidence.as_str(),
            function.incoming_call_count,
            function.outgoing_call_count,
            function.summary.block_count,
            function.summary.instruction_count
        );
    }
    println!(
        "+----------------+----------+--------+----------+----------+--------+--------------+"
    );

    if call_graph.edges.is_empty() && call_graph.external_edges.is_empty() {
        println!("\n  No function calls resolved");
        return;
    }

    if import_thunk_count > 0 {
        println!("\n[IMPORT THUNKS]");
        for function in call_graph
            .functions
            .iter()
            .filter(|function| function.import_thunk.is_some())
            .take(40)
        {
            let target = function.import_thunk.as_ref().unwrap();
            println!(
                "  {} => {} ({})",
                function.summary.entry, target.label, target.address
            );
        }
        if import_thunk_count > 40 {
            println!("  ... {} more import thunks", import_thunk_count - 40);
        }
    }

    if !call_graph.edges.is_empty() {
        println!("\n[INTERNAL CALL EDGES]");
        for edge in call_graph.edges.iter().take(40) {
            let sites = edge
                .call_sites
                .iter()
                .take(4)
                .map(|site| format!("{}", site))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {} -> {}  sites: {}", edge.caller, edge.callee, sites);
        }
        if call_graph.edges.len() > 40 {
            println!(
                "  ... {} more internal call edges",
                call_graph.edges.len() - 40
            );
        }
    }

    if !call_graph.external_edges.is_empty() {
        println!("\n[EXTERNAL IMPORT CALL EDGES]");
        for edge in call_graph.external_edges.iter().take(40) {
            let sites = edge
                .call_sites
                .iter()
                .take(4)
                .map(|site| format!("{}", site))
                .collect::<Vec<_>>()
                .join(", ");
            println!(
                "  {} -> {} ({})  sites: {}",
                edge.caller, edge.label, edge.target, sites
            );
        }
        if call_graph.external_edges.len() > 40 {
            println!(
                "  ... {} more external import call edges",
                call_graph.external_edges.len() - 40
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use disassembler::parser::{
        BinaryFormat, ExportSummary, FunctionRangeSummary, SectionSummary, SymbolKind,
        SymbolSummary,
    };

    #[test]
    fn builds_noreturn_call_targets_from_import_names() {
        let analysis = BinaryAnalysis {
            imports: vec![
                disassembler::parser::ImportSummary {
                    address: Some(0x3000),
                    library: Some("kernel32.dll".to_string()),
                    name: "ExitProcess".to_string(),
                },
                disassembler::parser::ImportSummary {
                    address: Some(0x3010),
                    library: Some("kernel32.dll".to_string()),
                    name: "CreateFileW".to_string(),
                },
                disassembler::parser::ImportSummary {
                    address: None,
                    library: None,
                    name: "abort".to_string(),
                },
            ],
            ..BinaryAnalysis::default()
        };

        let targets = build_noreturn_call_targets(&analysis);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].address, Address(0x3000));
        assert_eq!(targets[0].label, "kernel32.dll!ExitProcess");
    }
    #[test]
    fn builds_cfg_function_seeds_from_executable_binary_names() {
        let metadata = BinaryMetadata {
            format: BinaryFormat::PE,
            architecture: Architecture::X64,
            bitness: 64,
            endianness: disassembler::arch::Endianness::Little,
            entry_point: Some(0x1000),
        };
        let analysis = BinaryAnalysis {
            exports: vec![
                ExportSummary {
                    address: Some(0x1030),
                    name: "exported_function".to_string(),
                    kind: SymbolKind::Export,
                    size: 0,
                    forwarder: None,
                },
                ExportSummary {
                    address: Some(0x1040),
                    name: "forwarded_function".to_string(),
                    kind: SymbolKind::Export,
                    size: 0,
                    forwarder: Some("other.dll.forwarded_function".to_string()),
                },
                ExportSummary {
                    address: Some(0x2010),
                    name: "data_export".to_string(),
                    kind: SymbolKind::Export,
                    size: 0,
                    forwarder: None,
                },
            ],
            symbols: vec![
                SymbolSummary {
                    address: Some(0x1020),
                    name: "named_function".to_string(),
                    kind: SymbolKind::Function,
                },
                SymbolSummary {
                    address: Some(0x1050),
                    name: "named_object".to_string(),
                    kind: SymbolKind::Object,
                },
                SymbolSummary {
                    address: Some(0x2020),
                    name: "data_function_symbol".to_string(),
                    kind: SymbolKind::Function,
                },
            ],
            function_ranges: vec![
                FunctionRangeSummary {
                    start: 0x1060,
                    end: 0x1080,
                    section: Some(".text".to_string()),
                    source: "PE unwind".to_string(),
                    unwind_info: Some(0x3000),
                },
                FunctionRangeSummary {
                    start: 0x2030,
                    end: 0x2040,
                    section: Some(".rdata".to_string()),
                    source: "PE unwind".to_string(),
                    unwind_info: Some(0x3040),
                },
            ],
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x100,
                    file_offset: 0x400,
                    file_size: 0x100,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".rdata".to_string(),
                    address: 0x2000,
                    virtual_size: 0x80,
                    file_offset: 0x800,
                    file_size: 0x80,
                    readable: true,
                    writable: false,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        let seeds = build_cfg_function_seeds(&metadata, &analysis);

        assert_eq!(
            seeds,
            vec![
                FunctionSeed::entry_point(Address(0x1000)),
                FunctionSeed::symbol(Address(0x1020)),
                FunctionSeed::export(Address(0x1030)),
                FunctionSeed::unwind(Address(0x1060)),
            ]
        );
    }

    #[test]
    fn finds_section_selector_by_name_and_address() {
        let analysis = BinaryAnalysis {
            sections: vec![
                SectionSummary {
                    name: ".text".to_string(),
                    address: 0x1000,
                    virtual_size: 0x100,
                    file_offset: 0x400,
                    file_size: 0x100,
                    readable: true,
                    writable: false,
                    executable: true,
                },
                SectionSummary {
                    name: ".rdata".to_string(),
                    address: 0x2000,
                    virtual_size: 0x80,
                    file_offset: 0x800,
                    file_size: 0x80,
                    readable: true,
                    writable: false,
                    executable: false,
                },
            ],
            ..BinaryAnalysis::default()
        };

        assert_eq!(
            find_section_selector(&analysis, ".TEXT").map(|section| section.name.as_str()),
            Some(".text")
        );
        assert_eq!(
            find_section_selector(&analysis, "0x1040").map(|section| section.name.as_str()),
            Some(".text")
        );
        assert_eq!(
            find_section_selector(&analysis, "8192").map(|section| section.name.as_str()),
            Some(".rdata")
        );
        assert!(find_section_selector(&analysis, ".missing").is_none());
    }
}
