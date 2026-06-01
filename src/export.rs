use crate::arch;
use crate::graph::ControlFlowGraph;
use crate::parser::{BinaryAnalysis, BinaryMetadata};
use crate::project::AnalysisProject;
use arch::x86::Instruction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
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
            user_name: None,
            comment: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExportAnnotations {
    pub names: HashMap<u64, String>,
    pub comments: HashMap<u64, String>,
}

impl ExportAnnotations {
    pub fn from_project(project: &AnalysisProject) -> Self {
        Self {
            names: project
                .user_names
                .iter()
                .map(|name| (name.address, name.name.clone()))
                .collect(),
            comments: project
                .comments
                .iter()
                .map(|comment| (comment.address, comment.text.clone()))
                .collect(),
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
    pub import_count: usize,
    pub symbol_count: usize,
    pub string_count: usize,
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
        Self::export_instructions_with_metadata_analysis_and_annotations(
            instructions,
            format,
            path,
            binary_metadata,
            binary_analysis,
            None,
        )
    }

    pub fn export_instructions_with_metadata_analysis_and_annotations(
        instructions: &[Instruction],
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
        binary_analysis: Option<&BinaryAnalysis>,
        annotations: Option<&ExportAnnotations>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exportable = exportable_instructions(instructions, annotations);
        let metadata = Self::create_metadata_with_analysis(
            &exportable,
            None,
            binary_metadata,
            binary_analysis,
        );
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
        Self::export_with_cfg_metadata_analysis_and_annotations(
            instructions,
            cfg,
            format,
            path,
            binary_metadata,
            binary_analysis,
            None,
        )
    }

    pub fn export_with_cfg_metadata_analysis_and_annotations(
        instructions: &[Instruction],
        cfg: &ControlFlowGraph,
        format: ExportFormat,
        path: &str,
        binary_metadata: Option<&BinaryMetadata>,
        binary_analysis: Option<&BinaryAnalysis>,
        annotations: Option<&ExportAnnotations>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exportable = exportable_instructions(instructions, annotations);
        let cfg_info = Some(CfgExportData {
            block_count: cfg.blocks.len(),
            edge_count: cfg.edges.len(),
            entry_points: cfg.blocks.keys().map(|addr| addr.0).collect(),
        });

        let metadata = Self::create_metadata_with_analysis(
            &exportable,
            cfg_info.as_ref(),
            binary_metadata,
            binary_analysis,
        );
        let export_data = ExportData {
            metadata,
            instructions: exportable,
            cfg_info,
        };

        match format {
            ExportFormat::Json => Self::export_json(&export_data, path),
            ExportFormat::Html => Self::export_html_with_cfg(&export_data, cfg, path),
            ExportFormat::Dot => Self::export_dot(cfg, path),
            _ => Self::export_instructions_with_metadata_analysis_and_annotations(
                instructions,
                format,
                path,
                binary_metadata,
                binary_analysis,
                annotations,
            ),
        }
    }

    fn create_metadata_with_analysis(
        instructions: &[ExportableInstruction],
        _cfg_info: Option<&CfgExportData>,
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
            symbol_count: binary_analysis
                .map(|analysis| analysis.symbols.len())
                .unwrap_or_default(),
            string_count: binary_analysis
                .map(|analysis| analysis.strings.len())
                .unwrap_or_default(),
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

        writeln!(file, "Address,Bytes,Mnemonic,Operands,Size,Name,Comment")?;

        for inst in instructions {
            writeln!(
                file,
                "{},{},{},{},{},{},{}",
                csv_field(&inst.address),
                csv_field(&inst.bytes_hex),
                csv_field(&inst.mnemonic),
                csv_field(&inst.operands),
                inst.size,
                csv_field(inst.user_name.as_deref().unwrap_or_default()),
                csv_field(inst.comment.as_deref().unwrap_or_default())
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
        writeln!(file, "| Address | Name | Bytes | Instruction | Comment |")?;
        writeln!(file, "|---------|------|-------|-------------|---------|")?;

        for inst in &data.instructions {
            writeln!(
                file,
                "| {} | {} | `{}` | `{}` | {} |",
                inst.address,
                inst.user_name.as_deref().unwrap_or_default(),
                inst.bytes_hex,
                inst.full_text,
                inst.comment.as_deref().unwrap_or_default()
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
            if let Some(name) = &inst.user_name {
                writeln!(file, "{}:", name)?;
            }
            let comment = inst
                .comment
                .as_deref()
                .map(|comment| format!(" ; {comment}"))
                .unwrap_or_default();
            writeln!(file, "{}: {}{}", inst.address, inst.full_text, comment)?;
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
                <th>Name</th>
                <th>Comment</th>
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
                <td>{}</td>
                <td>{}</td>
            </tr>
"#,
                inst.address,
                inst.bytes_hex,
                inst.mnemonic,
                inst.operands,
                inst.user_name.as_deref().unwrap_or_default(),
                inst.comment.as_deref().unwrap_or_default()
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
    export_auto_format_with_metadata_analysis_and_annotations(
        instructions,
        cfg,
        path,
        binary_metadata,
        binary_analysis,
        None,
    )
}

pub fn export_auto_format_with_metadata_analysis_and_annotations(
    instructions: &[Instruction],
    cfg: Option<&ControlFlowGraph>,
    path: &str,
    binary_metadata: Option<&BinaryMetadata>,
    binary_analysis: Option<&BinaryAnalysis>,
    annotations: Option<&ExportAnnotations>,
) -> Result<(), Box<dyn std::error::Error>> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or("Could not determine file extension")?;

    let format = ExportFormat::from_extension(extension)
        .ok_or(format!("Unsupported export format: {}", extension))?;

    match cfg {
        Some(cfg) => Exporter::export_with_cfg_metadata_analysis_and_annotations(
            instructions,
            cfg,
            format,
            path,
            binary_metadata,
            binary_analysis,
            annotations,
        ),
        None => Exporter::export_instructions_with_metadata_analysis_and_annotations(
            instructions,
            format,
            path,
            binary_metadata,
            binary_analysis,
            annotations,
        ),
    }
}

fn exportable_instructions(
    instructions: &[Instruction],
    annotations: Option<&ExportAnnotations>,
) -> Vec<ExportableInstruction> {
    instructions
        .iter()
        .map(|instruction| {
            let mut exportable = ExportableInstruction::from(instruction);
            if let Some(annotations) = annotations {
                exportable.user_name = annotations.names.get(&instruction.address).cloned();
                exportable.comment = annotations.comments.get(&instruction.address).cloned();
            }
            exportable
        })
        .collect()
}

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::AnalysisProject;

    fn instruction(address: u64, text: &str) -> Instruction {
        Instruction {
            address,
            bytes: vec![0x90],
            text: text.to_string(),
        }
    }

    #[test]
    fn export_annotations_are_built_from_project() {
        let mut project = AnalysisProject::from_binary("sample.exe", b"binary");
        project.set_user_name(0x1000, "entry");
        project.set_comment(0x1000, "starts here");

        let annotations = ExportAnnotations::from_project(&project);

        assert_eq!(
            annotations.names.get(&0x1000).map(String::as_str),
            Some("entry")
        );
        assert_eq!(
            annotations.comments.get(&0x1000).map(String::as_str),
            Some("starts here")
        );
    }

    #[test]
    fn json_export_includes_instruction_annotations() {
        let path = std::env::temp_dir().join(format!(
            "disassembler-export-annotations-{}.json",
            std::process::id()
        ));
        let mut annotations = ExportAnnotations::default();
        annotations.names.insert(0x1000, "entry".to_string());
        annotations
            .comments
            .insert(0x1000, "first instruction".to_string());

        Exporter::export_instructions_with_metadata_analysis_and_annotations(
            &[instruction(0x1000, "nop")],
            ExportFormat::Json,
            path.to_str().unwrap(),
            None,
            None,
            Some(&annotations),
        )
        .unwrap();

        let json = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["instructions"][0]["user_name"], "entry");
        assert_eq!(value["instructions"][0]["comment"], "first instruction");
    }

    #[test]
    fn csv_export_includes_instruction_annotations() {
        let path = std::env::temp_dir().join(format!(
            "disassembler-export-annotations-{}.csv",
            std::process::id()
        ));
        let mut annotations = ExportAnnotations::default();
        annotations.names.insert(0x1000, "entry".to_string());
        annotations
            .comments
            .insert(0x1000, "call, quoted".to_string());

        Exporter::export_instructions_with_metadata_analysis_and_annotations(
            &[instruction(0x1000, "nop")],
            ExportFormat::Csv,
            path.to_str().unwrap(),
            None,
            None,
            Some(&annotations),
        )
        .unwrap();

        let csv = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(csv.contains("Address,Bytes,Mnemonic,Operands,Size,Name,Comment"));
        assert!(csv.contains("entry"));
        assert!(csv.contains("\"call, quoted\""));
    }
}
