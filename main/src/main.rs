use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

use clap::Parser;

pub mod disassembler;
pub mod parser;
pub mod graph;
pub mod tui;
pub mod export; 

use disassembler::{DisasmOpts, disasm};
use parser::TextSection;
use graph::{ControlFlowGraph, Instruction as CfgInstruction, Address};
use export::{Exporter, ExportFormat, export_auto_format}; // Add this line

#[derive(Debug, Clone, Parser)]
pub struct Opts {
    #[clap(long)]
    raw: bool,

    #[clap(long)]
    cfg: bool,

    #[clap(long)]
    tui: bool,

    #[clap(long, short = 'o')]
    output: Option<PathBuf>,

    #[clap(long, short = 'f')]
    format: Option<String>,

    #[clap(name = "FILE", value_parser)]
    files: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    env_logger::init();

    let opts = Opts::parse();
    if opts.files.is_empty() {
        eprintln!("No files provided");
        std::process::exit(1);
    }

    for path in &opts.files {
        let data = {
            let mut f = BufReader::new(File::open(path)?);
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            buf
        };

        let TextSection { va, bytes } = parser::get_text_section(&data).unwrap_or_else(|e| {
            eprintln!("{}: failed to parse .text: {}", path.display(), e);
            std::process::exit(1);
        });

        println!(
            "Successfully loaded text section from {} at VA {:#x} with {} bytes",
            path.display(),
            va,
            bytes.len()
        );

        let disasm_opts = DisasmOpts {
            base_address: va,
            bitness: 64,
        };

        let instructions = disasm(&bytes, disasm_opts);
        
        let cfg = if opts.cfg {
            let cfg_instructions: Vec<CfgInstruction> = instructions.iter().map(|inst| {
                CfgInstruction {
                    address: Address(inst.address),
                    mnemonic: extract_mnemonic(&inst.text),
                    operands: extract_operands(&inst.text),
                    bytes: inst.bytes.clone(),
                }
            }).collect();

            let mut cfg = ControlFlowGraph::new();
            cfg.build_from_instructions(cfg_instructions);
            Some(cfg)
        } else {
            None
        };

        if let Some(output_path) = &opts.output {
            let export_result = if let Some(format_str) = &opts.format {
                let format = match format_str.to_lowercase().as_str() {
                    "json" => ExportFormat::Json,
                    "csv" => ExportFormat::Csv,
                    "html" => ExportFormat::Html,
                    "markdown" | "md" => ExportFormat::Markdown,
                    "dot" => ExportFormat::Dot,
                    "assembly" | "asm" => ExportFormat::Assembly,
                    _ => {
                        eprintln!("Unknown format: {}", format_str);
                        std::process::exit(1);
                    }
                };
                
                match &cfg {
                    Some(cfg) => Exporter::export_with_cfg(&instructions, cfg, format, output_path.to_str().unwrap()),
                    None => Exporter::export_instructions(&instructions, format, output_path.to_str().unwrap()),
                }
            } else {
                export_auto_format(&instructions, cfg.as_ref(), output_path.to_str().unwrap())
            };

            if let Err(e) = export_result {
                eprintln!("Export failed: {}", e);
                std::process::exit(1);
            }
        }

        if opts.tui {
            if let Err(e) = tui::run_tui(instructions, cfg) {
                eprintln!("TUI error: {}", e);
                std::process::exit(1);
            }
        } else if opts.cfg && opts.output.is_none() {
            if let Some(cfg) = &cfg {
                println!("\n=== Control Flow Analysis ===");
                cfg.display_ascii();
            }
        } else if opts.output.is_none() {
            for inst in instructions {
                println!("{}", inst);
            }
        }

        println!(); 
    }

    Ok(())
}

fn extract_mnemonic(instruction_text: &str) -> String {
    instruction_text.split_whitespace()
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

