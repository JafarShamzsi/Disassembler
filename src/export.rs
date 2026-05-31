use crate::arch;
use crate::graph::ControlFlowGraph;
use crate::parser::BinaryMetadata;
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CfgExportData {
    pub block_count: usize,
    pub edge_count: usize,
    pub entry_points: Vec<u64>,
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
        let exportable: Vec<ExportableInstruction> =
            instructions.iter().map(|i| i.into()).collect();

        let metadata = Self::create_metadata(&exportable, None, binary_metadata);
        let export_data = ExportData {
            metadata,
            instructions: exportable,
            cfg_info: None,
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
        let exportable: Vec<ExportableInstruction> =
            instructions.iter().map(|i| i.into()).collect();

        let cfg_info = Some(CfgExportData {
            block_count: cfg.blocks.len(),
            edge_count: cfg.edges.len(),
            entry_points: cfg.blocks.keys().map(|addr| addr.0).collect(),
        });

        let metadata = Self::create_metadata(&exportable, cfg_info.as_ref(), binary_metadata);
        let export_data = ExportData {
            metadata,
            instructions: exportable,
            cfg_info,
        };

        match format {
            ExportFormat::Json => Self::export_json(&export_data, path),
            ExportFormat::Html => Self::export_html_with_cfg(&export_data, cfg, path),
            ExportFormat::Dot => Self::export_dot(cfg, path),
            _ => {
                Self::export_instructions_with_metadata(instructions, format, path, binary_metadata)
            }
        }
    }

    fn create_metadata(
        instructions: &[ExportableInstruction],
        _cfg_info: Option<&CfgExportData>,
        binary_metadata: Option<&BinaryMetadata>,
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
        writeln!(file)?;

        if let Some(cfg_info) = &data.cfg_info {
            writeln!(file, "## Control Flow Information")?;
            writeln!(file, "- **Basic Blocks:** {}", cfg_info.block_count)?;
            writeln!(file, "- **Edges:** {}", cfg_info.edge_count)?;
            writeln!(file)?;
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
                .unwrap_or_else(|| "none".to_string())
        )
    }

    fn html_cfg_summary(cfg_info: &CfgExportData) -> String {
        format!(
            r#"    <div class="cfg-summary">
        <h2>Control Flow Graph</h2>
        <p><strong>Basic Blocks:</strong> {}</p>
        <p><strong>Edges:</strong> {}</p>
        <p><strong>Entry Points:</strong> {}</p>
    </div>
"#,
            cfg_info.block_count,
            cfg_info.edge_count,
            cfg_info.entry_points.len()
        )
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
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or("Could not determine file extension")?;

    let format = ExportFormat::from_extension(extension)
        .ok_or(format!("Unsupported export format: {}", extension))?;

    match cfg {
        Some(cfg) => {
            Exporter::export_with_cfg_and_metadata(instructions, cfg, format, path, binary_metadata)
        }
        None => {
            Exporter::export_instructions_with_metadata(instructions, format, path, binary_metadata)
        }
    }
}
