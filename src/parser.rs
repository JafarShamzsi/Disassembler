use anyhow::{bail, Result};
use goblin::elf::header::{EM_386, EM_AARCH64, EM_ARM, EM_MIPS, EM_RISCV, EM_X86_64};
use goblin::elf::sym::{self, STT_FUNC, STT_OBJECT};
use goblin::pe::header::{
    COFF_MACHINE_ARM, COFF_MACHINE_ARM64, COFF_MACHINE_ARMNT, COFF_MACHINE_X86, COFF_MACHINE_X86_64,
};
use goblin::Object;

use crate::arch::{Architecture, Endianness};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFormat {
    PE,
    ELF,
}

impl BinaryFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryFormat::PE => "PE",
            BinaryFormat::ELF => "ELF",
        }
    }
}

impl std::fmt::Display for BinaryFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct BinaryMetadata {
    pub format: BinaryFormat,
    pub architecture: Architecture,
    pub bitness: u32,
    pub endianness: Endianness,
    pub entry_point: Option<u64>,
}

pub struct TextSection<'a> {
    pub va: u64,
    pub bytes: &'a [u8],
}

pub struct AnalyzedBinary<'a> {
    pub metadata: BinaryMetadata,
    pub text: TextSection<'a>,
    pub analysis: BinaryAnalysis,
}

#[derive(Debug, Clone, Default)]
pub struct BinaryAnalysis {
    pub imports: Vec<ImportSummary>,
    pub symbols: Vec<SymbolSummary>,
    pub strings: Vec<StringSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub address: Option<u64>,
    pub library: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Object,
    Import,
    Export,
    Other,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Object => "object",
            SymbolKind::Import => "import",
            SymbolKind::Export => "export",
            SymbolKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolSummary {
    pub address: Option<u64>,
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringSummary {
    pub address: u64,
    pub value: String,
}

pub fn get_text_section(data: &[u8]) -> Result<TextSection<'_>> {
    Ok(analyze_binary(data)?.text)
}

pub fn analyze_binary(data: &[u8]) -> Result<AnalyzedBinary<'_>> {
    match Object::parse(data)? {
        Object::PE(pe) => analyze_pe(data, pe),
        Object::Elf(elf) => analyze_elf(data, elf),
        _ => bail!("unsupported binary format"),
    }
}

fn analyze_pe<'a>(data: &'a [u8], pe: goblin::pe::PE<'_>) -> Result<AnalyzedBinary<'a>> {
    let base = pe.image_base as u64;
    let machine = pe.header.coff_header.machine;
    let architecture = pe_architecture(machine)?;

    let section = pe
        .sections
        .iter()
        .find(|sect| sect.name().unwrap_or("") == ".text")
        .ok_or_else(|| anyhow::anyhow!(".text section not found"))?;

    let va = base + section.virtual_address as u64;
    let bytes = checked_slice(
        data,
        section.pointer_to_raw_data as usize,
        section.size_of_raw_data as usize,
        ".text",
    )?;

    Ok(AnalyzedBinary {
        metadata: BinaryMetadata {
            format: BinaryFormat::PE,
            architecture,
            bitness: if pe.is_64 { 64 } else { 32 },
            endianness: Endianness::Little,
            entry_point: Some(base + pe.entry as u64),
        },
        text: TextSection { va, bytes },
        analysis: analyze_pe_names(data, &pe, base),
    })
}

fn analyze_elf<'a>(data: &'a [u8], elf: goblin::elf::Elf<'_>) -> Result<AnalyzedBinary<'a>> {
    let architecture = elf_architecture(elf.header.e_machine, elf.is_64)?;
    let section = elf
        .section_headers
        .iter()
        .find(|section| elf.shdr_strtab.get_at(section.sh_name) == Some(".text"))
        .ok_or_else(|| anyhow::anyhow!(".text section not found"))?;

    let bytes = checked_slice(
        data,
        section.sh_offset as usize,
        section.sh_size as usize,
        ".text",
    )?;

    Ok(AnalyzedBinary {
        metadata: BinaryMetadata {
            format: BinaryFormat::ELF,
            architecture,
            bitness: if elf.is_64 { 64 } else { 32 },
            endianness: if elf.little_endian {
                Endianness::Little
            } else {
                Endianness::Big
            },
            entry_point: if elf.entry == 0 {
                None
            } else {
                Some(elf.entry)
            },
        },
        text: TextSection {
            va: section.sh_addr,
            bytes,
        },
        analysis: analyze_elf_names(data, &elf),
    })
}

fn analyze_pe_names(data: &[u8], pe: &goblin::pe::PE<'_>, base: u64) -> BinaryAnalysis {
    let imports = pe
        .imports
        .iter()
        .map(|import| ImportSummary {
            address: Some(base + import.offset as u64),
            library: Some(import.dll.to_string()),
            name: import.name.to_string(),
        })
        .collect();

    let mut symbols: Vec<SymbolSummary> = pe
        .exports
        .iter()
        .filter_map(|export| {
            let name = export.name?;
            Some(SymbolSummary {
                address: Some(base + export.rva as u64),
                name: name.to_string(),
                kind: SymbolKind::Export,
            })
        })
        .collect();

    symbols.extend(pe.imports.iter().map(|import| SymbolSummary {
        address: Some(base + import.offset as u64),
        name: format!("{}!{}", import.dll, import.name),
        kind: SymbolKind::Import,
    }));

    BinaryAnalysis {
        imports,
        symbols,
        strings: extract_section_strings(
            data,
            pe.sections.iter().filter_map(|section| {
                let name = section.name().unwrap_or("");
                if name.is_empty() {
                    return None;
                }

                Some((
                    base + section.virtual_address as u64,
                    section.pointer_to_raw_data as usize,
                    section.size_of_raw_data as usize,
                ))
            }),
        ),
    }
}

fn analyze_elf_names(data: &[u8], elf: &goblin::elf::Elf<'_>) -> BinaryAnalysis {
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    collect_elf_symbols(
        elf.dynsyms.iter(),
        &elf.dynstrtab,
        &mut symbols,
        Some(&mut imports),
    );
    collect_elf_symbols(elf.syms.iter(), &elf.strtab, &mut symbols, None);

    BinaryAnalysis {
        imports,
        symbols,
        strings: extract_section_strings(
            data,
            elf.section_headers.iter().filter_map(|section| {
                if section.sh_size == 0 {
                    return None;
                }

                Some((
                    section.sh_addr,
                    section.sh_offset as usize,
                    section.sh_size as usize,
                ))
            }),
        ),
    }
}

fn collect_elf_symbols<'a>(
    symbols_iter: impl Iterator<Item = goblin::elf::sym::Sym> + 'a,
    names: &goblin::strtab::Strtab<'_>,
    symbols: &mut Vec<SymbolSummary>,
    mut imports: Option<&mut Vec<ImportSummary>>,
) {
    for symbol in symbols_iter {
        let Some(name) = names.get_at(symbol.st_name) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }

        let is_import = sym::is_import(symbol.st_info, symbol.st_value);
        let kind = if is_import {
            SymbolKind::Import
        } else {
            match sym::st_type(symbol.st_info) {
                STT_FUNC => SymbolKind::Function,
                STT_OBJECT => SymbolKind::Object,
                _ => SymbolKind::Other,
            }
        };

        if is_import {
            if let Some(imports) = imports.as_deref_mut() {
                imports.push(ImportSummary {
                    address: None,
                    library: None,
                    name: name.to_string(),
                });
            }
        }

        symbols.push(SymbolSummary {
            address: if symbol.st_value == 0 {
                None
            } else {
                Some(symbol.st_value)
            },
            name: name.to_string(),
            kind,
        });
    }
}

fn extract_section_strings(
    data: &[u8],
    sections: impl Iterator<Item = (u64, usize, usize)>,
) -> Vec<StringSummary> {
    let mut strings = Vec::new();

    for (section_va, file_offset, size) in sections {
        let Some(end) = file_offset.checked_add(size) else {
            continue;
        };
        if file_offset >= data.len() || end > data.len() {
            continue;
        }

        strings.extend(extract_printable_strings(
            section_va,
            &data[file_offset..end],
            4,
        ));
    }

    strings.sort_by_key(|string| string.address);
    strings
}

pub fn extract_printable_strings(
    base_address: u64,
    bytes: &[u8],
    min_len: usize,
) -> Vec<StringSummary> {
    let mut strings = Vec::new();
    let mut start = 0;
    let mut current = Vec::new();

    for (idx, byte) in bytes.iter().copied().enumerate() {
        if is_printable_ascii(byte) {
            if current.is_empty() {
                start = idx;
            }
            current.push(byte);
            continue;
        }

        if current.len() >= min_len {
            strings.push(StringSummary {
                address: base_address + start as u64,
                value: String::from_utf8_lossy(&current).to_string(),
            });
        }
        current.clear();
    }

    if current.len() >= min_len {
        strings.push(StringSummary {
            address: base_address + start as u64,
            value: String::from_utf8_lossy(&current).to_string(),
        });
    }

    strings
}

fn is_printable_ascii(byte: u8) -> bool {
    matches!(byte, b'\t' | b' '..=b'~')
}

fn checked_slice<'a>(
    data: &'a [u8],
    start: usize,
    len: usize,
    section_name: &str,
) -> Result<&'a [u8]> {
    let end = start
        .checked_add(len)
        .ok_or_else(|| anyhow::anyhow!("{section_name} section range overflows usize"))?;

    if start >= data.len() || end > data.len() {
        bail!(
            "{} section range is outside the file: start={}, size={}, file_size={}",
            section_name,
            start,
            len,
            data.len()
        );
    }

    Ok(&data[start..end])
}

fn pe_architecture(machine: u16) -> Result<Architecture> {
    match machine {
        COFF_MACHINE_X86 => Ok(Architecture::X86),
        COFF_MACHINE_X86_64 => Ok(Architecture::X64),
        COFF_MACHINE_ARM | COFF_MACHINE_ARMNT => Ok(Architecture::ARM),
        COFF_MACHINE_ARM64 => Ok(Architecture::AArch64),
        _ => bail!("unsupported PE architecture machine={:#x}", machine),
    }
}

fn elf_architecture(machine: u16, is_64: bool) -> Result<Architecture> {
    match machine {
        EM_386 => Ok(Architecture::X86),
        EM_X86_64 => Ok(Architecture::X64),
        EM_ARM => Ok(Architecture::ARM),
        EM_AARCH64 => Ok(Architecture::AArch64),
        EM_MIPS => {
            if is_64 {
                Ok(Architecture::MIPS64)
            } else {
                Ok(Architecture::MIPS)
            }
        }
        EM_RISCV => {
            if is_64 {
                Ok(Architecture::RISCV64)
            } else {
                Ok(Architecture::RISCV32)
            }
        }
        _ => bail!("unsupported ELF architecture machine={:#x}", machine),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_printable_ascii_strings_with_addresses() {
        let bytes = b"\0abc\0hello world\0rust\xfftooling";

        let strings = extract_printable_strings(0x1000, bytes, 4);

        assert_eq!(
            strings,
            vec![
                StringSummary {
                    address: 0x1005,
                    value: "hello world".to_string()
                },
                StringSummary {
                    address: 0x1011,
                    value: "rust".to_string()
                },
                StringSummary {
                    address: 0x1016,
                    value: "tooling".to_string()
                }
            ]
        );
    }

    #[test]
    fn analyzes_pe_imports_symbols_and_strings() {
        let data = include_bytes!("../tests/notepad.exe");

        let analyzed = analyze_binary(data).expect("test PE should parse");

        assert_eq!(analyzed.metadata.format, BinaryFormat::PE);
        assert!(!analyzed.analysis.imports.is_empty());
        assert!(!analyzed.analysis.symbols.is_empty());
        assert!(!analyzed.analysis.strings.is_empty());
        assert!(analyzed
            .analysis
            .imports
            .iter()
            .any(|import| import.library.is_some() && !import.name.is_empty()));
    }
}
