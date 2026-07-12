use crate::arch;
use crate::graph::{Address, ControlFlowGraph, ExternalCallTarget};
use crate::parser::{BinaryAnalysis, BinaryMetadata};
use arch::x86::Instruction;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
    Html,
    Markdown,
    Dot,
    Assembly,
}

impl ExportFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "csv" => Some(Self::Csv),
            "html" | "htm" => Some(Self::Html),
            "md" | "markdown" => Some(Self::Markdown),
            "dot" => Some(Self::Dot),
            "asm" | "s" => Some(Self::Assembly),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Html => "html",
            Self::Markdown => "md",
            Self::Dot => "dot",
            Self::Assembly => "asm",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportableInstruction {
    pub address: String,
    pub address_hex: u64,
    pub bytes: Vec<u8>,
    pub bytes_hex: String,
    pub mnemonic: String,
    pub operands: String,
    pub full_text: String,
    pub size: usize,
}

impl From<&Instruction> for ExportableInstruction {
    fn from(inst: &Instruction) -> Self {
        let parts: Vec<&str> = inst.text.splitn(2, ' ').collect();
        let mnemonic = parts[0].to_string();
        let operands = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };

        Self {
            address: format!("{:#08x}", inst.address),
            address_hex: inst.address,
            bytes: inst.bytes.clone(),
            bytes_hex: inst
                .bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" "),
            mnemonic,
            operands,
            full_text: inst.text.clone(),
            size: inst.bytes.len(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub metadata: ExportMetadata,
    pub instructions: Vec<ExportableInstruction>,
    pub cfg_info: Option<CfgExportData>,
    pub call_graph_info: Option<CallGraphExportData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub tool: String,
    pub version: String,
    pub timestamp: String,
    pub instruction_count: usize,
    pub address_range: (u64, u64),
    pub architecture: String,
    pub binary_format: Option<String>,
    pub entry_point: Option<u64>,
    pub import_count: usize,
    pub export_count: usize,
    pub symbol_count: usize,
    pub string_count: usize,
    pub relocation_count: usize,
    pub function_range_count: usize,
    pub section_count: usize,
    pub function_count: usize,
    pub call_graph_edge_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CfgExportData {
    pub block_count: usize,
    pub edge_count: usize,
    pub noreturn_target_count: usize,
    pub entry_points: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraphExportData {
    pub function_count: usize,
    pub edge_count: usize,
    pub internal_edge_count: usize,
    pub external_function_count: usize,
    pub external_edge_count: usize,
    pub functions: Vec<CallGraphFunctionExport>,
    pub edges: Vec<CallGraphEdgeExport>,
    pub external_functions: Vec<CallGraphExternalFunctionExport>,
    pub external_edges: Vec<CallGraphExternalEdgeExport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraphFunctionExport {
    pub entry: u64,
    pub kind: String,
    pub confidence: String,
    pub block_count: usize,
    pub instruction_count: usize,
    pub internal_edge_count: usize,
    pub incoming_call_count: usize,
    pub outgoing_call_count: usize,
    pub import_thunk_target: Option<u64>,
    pub import_thunk_label: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraphEdgeExport {
    pub caller: u64,
    pub callee: u64,
    pub call_sites: Vec<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraphExternalFunctionExport {
    pub address: u64,
    pub label: String,
    pub incoming_call_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraphExternalEdgeExport {
    pub caller: u64,
    pub target: u64,
    pub label: String,
    pub call_sites: Vec<u64>,
}

pub struct Exporter;

impl Exporter {
    pub fn export_instructions(
        instructions: &[Instruction],
        format: ExportFormat,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::export_instructions_with_metadata(instructions, format, path, None)
    }

    pub fn export_instructions_with_metadata(
        instructions: &[Instruction],
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::export_instructions_with_metadata_and_analysis(
            instructions,
            format,
            path,
            binary_metadata,
            None,
        )
    }

    pub fn export_instructions_with_metadata_and_analysis(
        instructions: &[Instruction],
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
        binary_analysis: Option<&BinaryAnalysis>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exportable: Vec<ExportableInstruction> =
            instructions.iter().map(|i| i.into()).collect();

        let metadata = Self::create_metadata_with_analysis(
            &exportable,
            None,
            None,
            binary_metadata,
            binary_analysis,
        );
        let export_data = ExportData {
            metadata,
            instructions: exportable,
            cfg_info: None,
            call_graph_info: None,
        };

        match format {
            ExportFormat::Json => Self::export_json(&export_data, path),
            ExportFormat::Csv => Self::export_csv(&export_data.instructions, path),
            ExportFormat::Html => Self::export_html(&export_data, path),
            ExportFormat::Markdown => Self::export_markdown(&export_data, path),
            ExportFormat::Assembly => Self::export_assembly(&export_data.instructions, path),
            ExportFormat::Dot => {
                Err("Dot format requires CFG data. Use export_cfg instead.".into())
            }
        }
    }

    pub fn export_with_cfg(
        instructions: &[Instruction],
        cfg: &ControlFlowGraph,
        format: ExportFormat,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::export_with_cfg_and_metadata(instructions, cfg, format, path, None)
    }

    pub fn export_with_cfg_and_metadata(
        instructions: &[Instruction],
        cfg: &ControlFlowGraph,
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::export_with_cfg_metadata_and_analysis(
            instructions,
            cfg,
            format,
            path,
            binary_metadata,
            None,
        )
    }

    pub fn export_with_cfg_metadata_and_analysis(
        instructions: &[Instruction],
        cfg: &ControlFlowGraph,
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
        binary_analysis: Option<&BinaryAnalysis>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exportable: Vec<ExportableInstruction> =
            instructions.iter().map(|i| i.into()).collect();

        let cfg_info = Some(CfgExportData {
            block_count: cfg.blocks.len(),
            edge_count: cfg.edges.len(),
            noreturn_target_count: cfg.noreturn_call_target_count(),
            entry_points: cfg.blocks.keys().map(|addr| addr.0).collect(),
        });
        let call_graph_info = Some(Self::create_call_graph_export(cfg, binary_analysis));

        let metadata = Self::create_metadata_with_analysis(
            &exportable,
            cfg_info.as_ref(),
            call_graph_info.as_ref(),
            binary_metadata,
            binary_analysis,
        );
        let export_data = ExportData {
            metadata,
            instructions: exportable,
            cfg_info,
            call_graph_info,
        };

        match format {
            ExportFormat::Json => Self::export_json(&export_data, path),
            ExportFormat::Html => Self::export_html_with_cfg(&export_data, cfg, path),
            ExportFormat::Markdown => Self::export_markdown(&export_data, path),
            ExportFormat::Dot => Self::export_dot(cfg, path),
            _ => Self::export_instructions_with_metadata_and_analysis(
                instructions,
                format,
                path,
                binary_metadata,
                binary_analysis,
            ),
        }
    }

    fn create_metadata_with_analysis(
        instructions: &[ExportableInstruction],
        _cfg_info: Option<&CfgExportData>,
        call_graph_info: Option<&CallGraphExportData>,
        binary_metadata: Option<&BinaryMetadata>,
        binary_analysis: Option<&BinaryAnalysis>,
    ) -> ExportMetadata {
        let (min_addr, max_addr) = if instructions.is_empty() {
            (0, 0)
        } else {
            let addresses: Vec<u64> = instructions.iter().map(|i| i.address_hex).collect();
            (
                *addresses.iter().min().unwrap(),
                *addresses.iter().max().unwrap(),
            )
        };

        ExportMetadata {
            tool: "Sharingan DisAssembler".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            instruction_count: instructions.len(),
            address_range: (min_addr, max_addr),
            architecture: binary_metadata
                .map(|metadata| metadata.architecture.to_string())
                .unwrap_or_else(|| "x86_64".to_string()),
            binary_format: binary_metadata.map(|metadata| metadata.format.to_string()),
            entry_point: binary_metadata.and_then(|metadata| metadata.entry_point),
            import_count: binary_analysis
                .map(|analysis| analysis.imports.len())
                .unwrap_or_default(),
            export_count: binary_analysis
                .map(|analysis| analysis.exports.len())
                .unwrap_or_default(),
            symbol_count: binary_analysis
                .map(|analysis| analysis.symbols.len())
                .unwrap_or_default(),
            string_count: binary_analysis
                .map(|analysis| analysis.strings.len())
                .unwrap_or_default(),
            relocation_count: binary_analysis
                .map(|analysis| analysis.relocations.len())
                .unwrap_or_default(),
            function_range_count: binary_analysis
                .map(|analysis| analysis.function_ranges.len())
                .unwrap_or_default(),
            section_count: binary_analysis
                .map(|analysis| analysis.sections.len())
                .unwrap_or_default(),
            function_count: call_graph_info
                .map(|call_graph| call_graph.function_count)
                .unwrap_or_default(),
            call_graph_edge_count: call_graph_info
                .map(|call_graph| call_graph.edge_count)
                .unwrap_or_default(),
        }
    }

    fn create_call_graph_export(
        cfg: &ControlFlowGraph,
        binary_analysis: Option<&BinaryAnalysis>,
    ) -> CallGraphExportData {
        let call_graph = cfg.call_graph_with_external_targets(
            binary_analysis
                .map(build_external_call_targets)
                .unwrap_or_default(),
        );
        let functions = call_graph
            .functions
            .iter()
            .map(|function| CallGraphFunctionExport {
                entry: function.summary.entry.0,
                kind: function.summary.kind.as_str().to_string(),
                confidence: function.summary.confidence.as_str().to_string(),
                block_count: function.summary.block_count,
                instruction_count: function.summary.instruction_count,
                internal_edge_count: function.summary.edge_count,
                incoming_call_count: function.incoming_call_count,
                outgoing_call_count: function.outgoing_call_count,
                import_thunk_target: function
                    .import_thunk
                    .as_ref()
                    .map(|target| target.address.0),
                import_thunk_label: function
                    .import_thunk
                    .as_ref()
                    .map(|target| target.label.clone()),
            })
            .collect::<Vec<_>>();
        let edges = call_graph
            .edges
            .iter()
            .map(|edge| CallGraphEdgeExport {
                caller: edge.caller.0,
                callee: edge.callee.0,
                call_sites: edge.call_sites.iter().map(|site| site.0).collect(),
            })
            .collect::<Vec<_>>();
        let external_functions = call_graph
            .external_functions
            .iter()
            .map(|function| CallGraphExternalFunctionExport {
                address: function.address.0,
                label: function.label.clone(),
                incoming_call_count: function.incoming_call_count,
            })
            .collect::<Vec<_>>();
        let external_edges = call_graph
            .external_edges
            .iter()
            .map(|edge| CallGraphExternalEdgeExport {
                caller: edge.caller.0,
                target: edge.target.0,
                label: edge.label.clone(),
                call_sites: edge.call_sites.iter().map(|site| site.0).collect(),
            })
            .collect::<Vec<_>>();

        CallGraphExportData {
            function_count: functions.len(),
            edge_count: edges.len() + external_edges.len(),
            internal_edge_count: edges.len(),
            external_function_count: external_functions.len(),
            external_edge_count: external_edges.len(),
            functions,
            edges,
            external_functions,
            external_edges,
        }
    }

    fn export_json(data: &ExportData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(data)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        println!(
            "Exported {} instructions to JSON: {}",
            data.instructions.len(),
            path
        );
        Ok(())
    }

    fn export_csv(
        instructions: &[ExportableInstruction],
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        writeln!(file, "Address,Bytes,Mnemonic,Operands,Size")?;

        for inst in instructions {
            writeln!(
                file,
                "{},{},{},{},{}",
                inst.address, inst.bytes_hex, inst.mnemonic, inst.operands, inst.size
            )?;
        }

        println!(
            "Exported {} instructions to CSV: {}",
            instructions.len(),
            path
        );
        Ok(())
    }

    fn export_html(data: &ExportData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        write!(file, "{}", Self::html_header(&data.metadata))?;
        write!(
            file,
            "{}",
            Self::html_instructions_table(&data.instructions)
        )?;
        write!(file, "{}", Self::html_footer())?;

        println!(
            "Exported {} instructions to HTML: {}",
            data.instructions.len(),
            path
        );
        Ok(())
    }

    fn export_html_with_cfg(
        data: &ExportData,
        cfg: &ControlFlowGraph,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        write!(file, "{}", Self::html_header(&data.metadata))?;

        if let Some(cfg_info) = &data.cfg_info {
            write!(file, "{}", Self::html_cfg_summary(cfg_info))?;
        }

        if let Some(call_graph_info) = &data.call_graph_info {
            write!(file, "{}", Self::html_call_graph_summary(call_graph_info))?;
        }

        write!(file, "{}", Self::html_cfg_blocks(cfg))?;
        write!(
            file,
            "{}",
            Self::html_instructions_table(&data.instructions)
        )?;
        write!(file, "{}", Self::html_footer())?;

        println!(
            "Exported {} instructions with CFG to HTML: {}",
            data.instructions.len(),
            path
        );
        Ok(())
    }

    fn export_markdown(data: &ExportData, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        writeln!(file, "# Disassembly Report")?;
        writeln!(file)?;
        writeln!(file, "**Generated by:** {}", data.metadata.tool)?;
        writeln!(file, "**Version:** {}", data.metadata.version)?;
        writeln!(file, "**Timestamp:** {}", data.metadata.timestamp)?;
        writeln!(
            file,
            "**Instructions:** {}",
            data.metadata.instruction_count
        )?;
        writeln!(
            file,
            "**Address Range:** {:#x} - {:#x}",
            data.metadata.address_range.0, data.metadata.address_range.1
        )?;
        writeln!(
            file,
            "**Imports / Exports / Symbols:** {} / {} / {}",
            data.metadata.import_count, data.metadata.export_count, data.metadata.symbol_count
        )?;
        writeln!(file, "**Relocations:** {}", data.metadata.relocation_count)?;
        writeln!(
            file,
            "**Unwind function ranges:** {}",
            data.metadata.function_range_count
        )?;
        writeln!(file)?;

        if let Some(cfg_info) = &data.cfg_info {
            writeln!(file, "## Control Flow Information")?;
            writeln!(file, "- **Basic Blocks:** {}", cfg_info.block_count)?;
            writeln!(file, "- **Edges:** {}", cfg_info.edge_count)?;
            writeln!(
                file,
                "- **Noreturn import targets:** {}",
                cfg_info.noreturn_target_count
            )?;
            writeln!(file)?;
        }

        if let Some(call_graph_info) = &data.call_graph_info {
            writeln!(file, "## Call Graph")?;
            writeln!(file, "- **Functions:** {}", call_graph_info.function_count)?;
            writeln!(file, "- **Call Edges:** {}", call_graph_info.edge_count)?;
            writeln!(
                file,
                "  - Internal: {}  External imports: {}",
                call_graph_info.internal_edge_count, call_graph_info.external_edge_count
            )?;
            writeln!(file)?;
            if !call_graph_info.functions.is_empty() {
                writeln!(file, "| Function | Kind | Confidence | Calls In | Calls Out | Blocks | Instructions | Import Thunk |")?;
                writeln!(file, "|----------|------|------------|----------|-----------|--------|--------------|--------------|")?;
                for function in call_graph_info.functions.iter().take(25) {
                    writeln!(
                        file,
                        "| {:#x} | {} | {} | {} | {} | {} | {} | {} |",
                        function.entry,
                        function.kind,
                        function.confidence,
                        function.incoming_call_count,
                        function.outgoing_call_count,
                        function.block_count,
                        function.instruction_count,
                        function.import_thunk_label.as_deref().unwrap_or("")
                    )?;
                }
                writeln!(file)?;
            }
            if !call_graph_info.edges.is_empty() {
                writeln!(file, "### Internal Calls")?;
                writeln!(file)?;
                writeln!(file, "| Caller | Callee | Call Sites |")?;
                writeln!(file, "|--------|--------|------------|")?;
                for edge in call_graph_info.edges.iter().take(25) {
                    let sites = edge
                        .call_sites
                        .iter()
                        .take(4)
                        .map(|site| format!("{site:#x}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        file,
                        "| {:#x} | {:#x} | {} |",
                        edge.caller, edge.callee, sites
                    )?;
                }
                writeln!(file)?;
            }
            if !call_graph_info.external_edges.is_empty() {
                writeln!(file, "### External Import Calls")?;
                writeln!(file)?;
                writeln!(file, "| Caller | Import | Target | Call Sites |")?;
                writeln!(file, "|--------|--------|--------|------------|")?;
                for edge in call_graph_info.external_edges.iter().take(25) {
                    let sites = edge
                        .call_sites
                        .iter()
                        .take(4)
                        .map(|site| format!("{site:#x}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        file,
                        "| {:#x} | {} | {:#x} | {} |",
                        edge.caller, edge.label, edge.target, sites
                    )?;
                }
                writeln!(file)?;
            }
        }

        writeln!(file, "## Instructions")?;
        writeln!(file)?;
        writeln!(file, "| Address | Bytes | Instruction |")?;
        writeln!(file, "|---------|-------|-------------|")?;

        for inst in &data.instructions {
            writeln!(
                file,
                "| {} | `{}` | `{}` |",
                inst.address, inst.bytes_hex, inst.full_text
            )?;
        }

        println!(
            "Exported {} instructions to Markdown: {}",
            data.instructions.len(),
            path
        );
        Ok(())
    }

    fn export_assembly(
        instructions: &[ExportableInstruction],
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        writeln!(file, "; Generated by DeCompiler")?;
        writeln!(file, "; {} instructions", instructions.len())?;
        writeln!(file)?;

        for inst in instructions {
            writeln!(file, "{}: {}", inst.address, inst.full_text)?;
        }

        println!(
            "Exported {} instructions to Assembly: {}",
            instructions.len(),
            path
        );
        Ok(())
    }

    fn export_dot(cfg: &ControlFlowGraph, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(path)?;

        writeln!(file, "digraph CFG {{")?;
        writeln!(
            file,
            "  node [shape=box, fontname=\"Courier\", fontsize=10];"
        )?;
        writeln!(file, "  edge [fontname=\"Arial\", fontsize=8];")?;
        writeln!(file)?;

        for (addr, block) in &cfg.blocks {
            let label = format!(
                "Block {}\\n{}",
                addr,
                block
                    .instructions
                    .iter()
                    .take(5) // Limit to first 5 instructions for readability
                    .map(|i| format!("{}", i))
                    .collect::<Vec<_>>()
                    .join("\\n")
            );

            writeln!(file, "  \"{}\" [label=\"{}\"];", addr, label)?;
        }

        writeln!(file)?;

        for edge in &cfg.edges {
            let (label, color) = match edge.edge_type {
                crate::graph::EdgeType::ConditionalTrue => ("T", "green"),
                crate::graph::EdgeType::ConditionalFalse => ("F", "red"),
                crate::graph::EdgeType::Call => ("call", "blue"),
                crate::graph::EdgeType::Return => ("ret", "purple"),
                crate::graph::EdgeType::Unconditional => ("", "black"),
            };

            writeln!(
                file,
                "  \"{}\" -> \"{}\" [label=\"{}\" color=\"{}\"];",
                edge.from, edge.to, label, color
            )?;
        }

        writeln!(file, "}}")?;

        println!("Exported CFG to DOT format: {}", path);
        println!(
            "Generate image with: dot -Tpng {} -o {}.png",
            path,
            Path::new(path).file_stem().unwrap().to_str().unwrap()
        );
        Ok(())
    }

    fn html_header(metadata: &ExportMetadata) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Disassembly Report</title>
    <style>
        body {{ font-family: 'Courier New', monospace; margin: 20px; background-color: #1e1e1e; color: #d4d4d4; }}
        .header {{ background: #2d2d30; padding: 20px; border-radius: 5px; margin-bottom: 20px; }}
        .metadata {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; }}
        .metadata-item {{ background: #3c3c3c; padding: 10px; border-radius: 3px; }}
        table {{ width: 100%; border-collapse: collapse; background: #2d2d30; }}
        th, td {{ padding: 8px; text-align: left; border-bottom: 1px solid #404040; }}
        th {{ background-color: #404040; font-weight: bold; }}
        .address {{ color: #569cd6; }}
        .bytes {{ color: #ce9178; }}
        .mnemonic {{ color: #4ec9b0; font-weight: bold; }}
        .operands {{ color: #9cdcfe; }}
        .cfg-summary {{ background: #2d2d30; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .basic-block {{ background: #3c3c3c; margin: 10px 0; padding: 10px; border-radius: 3px; border-left: 3px solid #007acc; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Disassembly Report</h1>
        <div class="metadata">
            <div class="metadata-item"><strong>Tool:</strong> {}</div>
            <div class="metadata-item"><strong>Version:</strong> {}</div>
            <div class="metadata-item"><strong>Generated:</strong> {}</div>
            <div class="metadata-item"><strong>Instructions:</strong> {}</div>
            <div class="metadata-item"><strong>Address Range:</strong> {:#x} - {:#x}</div>
            <div class="metadata-item"><strong>Architecture:</strong> {}</div>
            <div class="metadata-item"><strong>Format:</strong> {}</div>
            <div class="metadata-item"><strong>Entry Point:</strong> {}</div>
            <div class="metadata-item"><strong>Imports:</strong> {}</div>
            <div class="metadata-item"><strong>Exports:</strong> {}</div>
            <div class="metadata-item"><strong>Symbols:</strong> {}</div>
            <div class="metadata-item"><strong>Relocations:</strong> {}</div>
            <div class="metadata-item"><strong>Unwind ranges:</strong> {}</div>
            <div class="metadata-item"><strong>Sections:</strong> {}</div>
            <div class="metadata-item"><strong>Functions:</strong> {}</div>
            <div class="metadata-item"><strong>Call Edges:</strong> {}</div>
        </div>
    </div>
"#,
            metadata.tool,
            metadata.version,
            metadata.timestamp,
            metadata.instruction_count,
            metadata.address_range.0,
            metadata.address_range.1,
            metadata.architecture,
            metadata.binary_format.as_deref().unwrap_or("unknown"),
            metadata
                .entry_point
                .map(|entry| format!("{entry:#x}"))
                .unwrap_or_else(|| "none".to_string()),
            metadata.import_count,
            metadata.export_count,
            metadata.symbol_count,
            metadata.relocation_count,
            metadata.function_range_count,
            metadata.section_count,
            metadata.function_count,
            metadata.call_graph_edge_count
        )
    }

    fn html_cfg_summary(cfg_info: &CfgExportData) -> String {
        format!(
            r#"    <div class="cfg-summary">
        <h2>Control Flow Graph</h2>
        <p><strong>Basic Blocks:</strong> {}</p>
        <p><strong>Edges:</strong> {}</p>
        <p><strong>Noreturn import targets:</strong> {}</p>
        <p><strong>Entry Points:</strong> {}</p>
    </div>
"#,
            cfg_info.block_count,
            cfg_info.edge_count,
            cfg_info.noreturn_target_count,
            cfg_info.entry_points.len()
        )
    }

    fn html_call_graph_summary(call_graph_info: &CallGraphExportData) -> String {
        let mut html = format!(
            r#"    <div class="cfg-summary">
        <h2>Call Graph</h2>
        <p><strong>Functions:</strong> {}</p>
        <p><strong>Call Edges:</strong> {} internal, {} external imports</p>
"#,
            call_graph_info.function_count,
            call_graph_info.internal_edge_count,
            call_graph_info.external_edge_count
        );

        if !call_graph_info.functions.is_empty() {
            html.push_str("        <table><thead><tr><th>Function</th><th>Kind</th><th>Confidence</th><th>Calls In</th><th>Calls Out</th><th>Blocks</th><th>Instructions</th><th>Import Thunk</th></tr></thead><tbody>\n");
            for function in call_graph_info.functions.iter().take(20) {
                html.push_str(&format!(
                    "            <tr><td>{:#x}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                    function.entry,
                    html_escape(&function.kind),
                    html_escape(&function.confidence),
                    function.incoming_call_count,
                    function.outgoing_call_count,
                    function.block_count,
                    function.instruction_count,
                    html_escape(function.import_thunk_label.as_deref().unwrap_or(""))
                ));
            }
            html.push_str("        </tbody></table>\n");
        }

        if !call_graph_info.edges.is_empty() {
            html.push_str("        <h3>Internal Calls</h3>\n        <ul>\n");
            for edge in call_graph_info.edges.iter().take(10) {
                let sites = edge
                    .call_sites
                    .iter()
                    .take(3)
                    .map(|site| format!("{site:#x}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                html.push_str(&format!(
                    "            <li>{:#x} -> {:#x} ({})</li>\n",
                    edge.caller, edge.callee, sites
                ));
            }
            html.push_str("        </ul>\n");
        }

        if !call_graph_info.external_edges.is_empty() {
            html.push_str("        <h3>External Import Calls</h3>\n        <ul>\n");
            for edge in call_graph_info.external_edges.iter().take(10) {
                let sites = edge
                    .call_sites
                    .iter()
                    .take(3)
                    .map(|site| format!("{site:#x}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                html.push_str(&format!(
                    "            <li>{:#x} -> {} ({:#x}) ({})</li>\n",
                    edge.caller,
                    html_escape(&edge.label),
                    edge.target,
                    sites
                ));
            }
            html.push_str("        </ul>\n");
        }

        html.push_str("    </div>\n");
        html
    }

    fn html_cfg_blocks(cfg: &ControlFlowGraph) -> String {
        let mut html = String::from("    <h2>Basic Blocks</h2>\n");

        let mut sorted_blocks: Vec<_> = cfg.blocks.iter().collect();
        sorted_blocks.sort_by_key(|(addr, _)| addr.0);

        for (addr, block) in sorted_blocks.iter().take(10) {
            // Limit for performance
            html.push_str(&format!(
                r#"    <div class="basic-block">
        <h3>Block {}</h3>
        <ul>
"#,
                addr
            ));

            for inst in &block.instructions {
                html.push_str(&format!("            <li>{}</li>\n", inst));
            }

            if !block.successors.is_empty() {
                html.push_str(&format!(
                    "        </ul>
        <p><strong>Successors:</strong> {:?}</p>
    </div>
",
                    block.successors
                ));
            } else {
                html.push_str("        </ul>\n    </div>\n");
            }
        }

        html
    }

    fn html_instructions_table(instructions: &[ExportableInstruction]) -> String {
        let mut html = String::from(
            r#"    <h2>Instructions</h2>
    <table>
        <thead>
            <tr>
                <th>Address</th>
                <th>Bytes</th>
                <th>Mnemonic</th>
                <th>Operands</th>
            </tr>
        </thead>
        <tbody>
"#,
        );

        for inst in instructions {
            html.push_str(&format!(
                r#"            <tr>
                <td class="address">{}</td>
                <td class="bytes">{}</td>
                <td class="mnemonic">{}</td>
                <td class="operands">{}</td>
            </tr>
"#,
                inst.address, inst.bytes_hex, inst.mnemonic, inst.operands
            ));
        }

        html.push_str("        </tbody>\n    </table>\n");
        html
    }

    fn html_footer() -> &'static str {
        "</body>\n</html>"
    }
}

pub fn export_auto_format(
    instructions: &[Instruction],
    cfg: Option<&ControlFlowGraph>,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    export_auto_format_with_metadata(instructions, cfg, path, None)
}

pub fn export_auto_format_with_metadata(
    instructions: &[Instruction],
    cfg: Option<&ControlFlowGraph>,
    path: &str,
    binary_metadata: Option<&BinaryMetadata>,
) -> Result<(), Box<dyn std::error::Error>> {
    export_auto_format_with_metadata_and_analysis(instructions, cfg, path, binary_metadata, None)
}

pub fn export_auto_format_with_metadata_and_analysis(
    instructions: &[Instruction],
    cfg: Option<&ControlFlowGraph>,
    path: &str,
    binary_metadata: Option<&BinaryMetadata>,
    binary_analysis: Option<&BinaryAnalysis>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or("Could not determine file extension")?;

    let format = ExportFormat::from_extension(extension)
        .ok_or(format!("Unsupported export format: {}", extension))?;

    match cfg {
        Some(cfg) => Exporter::export_with_cfg_metadata_and_analysis(
            instructions,
            cfg,
            format,
            path,
            binary_metadata,
            binary_analysis,
        ),
        None => Exporter::export_instructions_with_metadata_and_analysis(
            instructions,
            format,
            path,
            binary_metadata,
            binary_analysis,
        ),
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

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Address, FunctionSeed, Instruction as CfgInstruction};
    use crate::parser::ImportSummary;
    use std::fs;

    fn instruction(address: u64, text: &str, size: usize) -> Instruction {
        Instruction {
            address,
            bytes: vec![0x90; size],
            text: text.to_string(),
        }
    }

    fn cfg_instruction(
        address: u64,
        mnemonic: &str,
        operands: &str,
        size: usize,
    ) -> CfgInstruction {
        CfgInstruction {
            address: Address(address),
            mnemonic: mnemonic.to_string(),
            operands: operands.to_string(),
            bytes: vec![0x90; size],
        }
    }

    #[test]
    fn json_export_includes_noreturn_cfg_info() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds_and_noreturn_targets(
            vec![
                cfg_instruction(0x1000, "call", "qword [rel 0000000000003000h]", 6),
                cfg_instruction(0x1006, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
            [crate::graph::NoreturnCallTarget {
                address: Address(0x3000),
                label: "kernel32.dll!ExitProcess".to_string(),
            }],
        );

        let instructions = vec![
            instruction(0x1000, "call qword [rel 0000000000003000h]", 6),
            instruction(0x1006, "ret", 1),
        ];
        let path = std::env::temp_dir().join(format!(
            "disassembler-noreturn-cfg-export-{}.json",
            std::process::id()
        ));

        Exporter::export_with_cfg_metadata_and_analysis(
            &instructions,
            &cfg,
            ExportFormat::Json,
            path.to_str().unwrap(),
            None,
            None,
        )
        .unwrap();

        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("export should be valid JSON");
        let _ = fs::remove_file(&path);

        assert_eq!(json["cfg_info"]["noreturn_target_count"], 1);
        assert_eq!(json["cfg_info"]["edge_count"], 0);
    }
    #[test]
    fn json_export_includes_import_thunk_info() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                cfg_instruction(0x1000, "call", "0000000000002000h", 5),
                cfg_instruction(0x1005, "ret", "", 1),
                cfg_instruction(0x2000, "jmp", "qword [rel 0000000000003000h]", 6),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
        );

        let instructions = vec![
            instruction(0x1000, "call 0000000000002000h", 5),
            instruction(0x1005, "ret", 1),
            instruction(0x2000, "jmp qword [rel 0000000000003000h]", 6),
        ];
        let analysis = BinaryAnalysis {
            imports: vec![ImportSummary {
                address: Some(0x3000),
                library: Some("kernel32.dll".to_string()),
                name: "ExitProcess".to_string(),
            }],
            ..BinaryAnalysis::default()
        };
        let path = std::env::temp_dir().join(format!(
            "disassembler-import-thunk-export-{}.json",
            std::process::id()
        ));

        Exporter::export_with_cfg_metadata_and_analysis(
            &instructions,
            &cfg,
            ExportFormat::Json,
            path.to_str().unwrap(),
            None,
            Some(&analysis),
        )
        .unwrap();

        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("export should be valid JSON");
        let _ = fs::remove_file(&path);

        let thunk = json["call_graph_info"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["entry"] == 0x2000)
            .unwrap();
        assert_eq!(thunk["kind"], "thunk");
        assert_eq!(thunk["import_thunk_target"], 0x3000);
        assert_eq!(thunk["import_thunk_label"], "kernel32.dll!ExitProcess");
        assert!(json["call_graph_info"]["external_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["caller"] == 0x2000 && edge["target"] == 0x3000));
    }
    #[test]
    fn json_export_includes_external_import_call_graph_info() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions_with_function_seeds(
            vec![
                cfg_instruction(0x1000, "call", "qword [rel 0000000000003000h]", 6),
                cfg_instruction(0x1006, "ret", "", 1),
            ],
            [FunctionSeed::entry_point(Address(0x1000))],
        );

        let instructions = vec![
            instruction(0x1000, "call qword [rel 0000000000003000h]", 6),
            instruction(0x1006, "ret", 1),
        ];
        let analysis = BinaryAnalysis {
            imports: vec![ImportSummary {
                address: Some(0x3000),
                library: Some("kernel32.dll".to_string()),
                name: "ExitProcess".to_string(),
            }],
            ..BinaryAnalysis::default()
        };
        let path = std::env::temp_dir().join(format!(
            "disassembler-external-callgraph-export-{}.json",
            std::process::id()
        ));

        Exporter::export_with_cfg_metadata_and_analysis(
            &instructions,
            &cfg,
            ExportFormat::Json,
            path.to_str().unwrap(),
            None,
            Some(&analysis),
        )
        .unwrap();

        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("export should be valid JSON");
        let _ = fs::remove_file(&path);

        assert_eq!(json["metadata"]["function_count"], 1);
        assert_eq!(json["metadata"]["call_graph_edge_count"], 1);
        assert_eq!(json["call_graph_info"]["function_count"], 1);
        assert_eq!(json["call_graph_info"]["internal_edge_count"], 0);
        assert_eq!(json["call_graph_info"]["external_edge_count"], 1);
        assert_eq!(json["call_graph_info"]["external_function_count"], 1);
        assert_eq!(
            json["call_graph_info"]["external_functions"][0]["label"],
            "kernel32.dll!ExitProcess"
        );
        assert_eq!(
            json["call_graph_info"]["external_edges"][0]["caller"],
            0x1000
        );
        assert_eq!(
            json["call_graph_info"]["external_edges"][0]["target"],
            0x3000
        );
        assert_eq!(
            json["call_graph_info"]["external_edges"][0]["label"],
            "kernel32.dll!ExitProcess"
        );
        assert_eq!(
            json["call_graph_info"]["external_edges"][0]["call_sites"][0],
            0x1000
        );
    }

    #[test]
    fn json_export_includes_call_graph_info() {
        let mut cfg = ControlFlowGraph::new();
        cfg.build_from_instructions(vec![
            cfg_instruction(0x1000, "call", "0000000000002000h", 5),
            cfg_instruction(0x1005, "ret", "", 1),
            cfg_instruction(0x2000, "push", "rbp", 1),
            cfg_instruction(0x2001, "mov", "rbp,rsp", 3),
            cfg_instruction(0x2004, "ret", "", 1),
        ]);

        let instructions = vec![
            instruction(0x1000, "call 0000000000002000h", 5),
            instruction(0x1005, "ret", 1),
            instruction(0x2000, "push rbp", 1),
            instruction(0x2001, "mov rbp,rsp", 3),
            instruction(0x2004, "ret", 1),
        ];
        let path = std::env::temp_dir().join(format!(
            "disassembler-callgraph-export-{}.json",
            std::process::id()
        ));

        Exporter::export_with_cfg_metadata_and_analysis(
            &instructions,
            &cfg,
            ExportFormat::Json,
            path.to_str().unwrap(),
            None,
            None,
        )
        .unwrap();

        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
            .expect("export should be valid JSON");
        let _ = fs::remove_file(&path);

        assert_eq!(json["metadata"]["function_count"], 2);
        assert_eq!(json["metadata"]["call_graph_edge_count"], 1);
        assert_eq!(json["call_graph_info"]["function_count"], 2);
        assert_eq!(json["call_graph_info"]["functions"][0]["kind"], "entry");
        assert_eq!(
            json["call_graph_info"]["functions"][0]["confidence"],
            "high"
        );
        assert_eq!(json["call_graph_info"]["functions"][1]["kind"], "standard");
        assert_eq!(
            json["call_graph_info"]["functions"][1]["confidence"],
            "high"
        );
        assert_eq!(json["call_graph_info"]["edges"][0]["caller"], 0x1000);
        assert_eq!(json["call_graph_info"]["edges"][0]["callee"], 0x2000);
        assert_eq!(json["call_graph_info"]["edges"][0]["call_sites"][0], 0x1000);
    }
}
