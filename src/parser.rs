use anyhow::{bail, Result};
use goblin::elf::header::{EM_386, EM_AARCH64, EM_ARM, EM_MIPS, EM_RISCV, EM_X86_64};
use goblin::elf::reloc;
use goblin::elf::sym::{
    self, STB_GLOBAL, STB_WEAK, STT_FUNC, STT_OBJECT, STV_DEFAULT, STV_PROTECTED,
};
use goblin::pe::export::Reexport;
use goblin::pe::header::{
    COFF_MACHINE_ARM, COFF_MACHINE_ARM64, COFF_MACHINE_ARMNT, COFF_MACHINE_X86, COFF_MACHINE_X86_64,
};
use goblin::Object;

use crate::arch::{Architecture, Endianness};

const MAX_RELOCATIONS: usize = 20_000;

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

#[derive(Debug, Clone)]
pub struct LoadedSection<'a> {
    pub name: String,
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
    pub exports: Vec<ExportSummary>,
    pub symbols: Vec<SymbolSummary>,
    pub strings: Vec<StringSummary>,
    pub data_objects: Vec<DataObjectSummary>,
    pub relocations: Vec<RelocationSummary>,
    pub function_ranges: Vec<FunctionRangeSummary>,
    pub sections: Vec<SectionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub address: Option<u64>,
    pub library: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSummary {
    pub address: Option<u64>,
    pub name: String,
    pub kind: SymbolKind,
    pub size: usize,
    pub forwarder: Option<String>,
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
    pub section: Option<String>,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataObjectKind {
    Pointer,
}

impl DataObjectKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataObjectKind::Pointer => "pointer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataObjectSummary {
    pub address: u64,
    pub section: Option<String>,
    pub kind: DataObjectKind,
    pub size: usize,
    pub value: u64,
    pub target: u64,
    pub target_section: Option<String>,
    pub target_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelocationSummary {
    pub address: u64,
    pub section: Option<String>,
    pub source: String,
    pub kind: String,
    pub type_id: u64,
    pub symbol: Option<String>,
    pub addend: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionRangeSummary {
    pub start: u64,
    pub end: u64,
    pub section: Option<String>,
    pub source: String,
    pub unwind_info: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSummary {
    pub name: String,
    pub address: u64,
    pub virtual_size: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl BinaryAnalysis {
    pub fn section_containing_address(&self, address: u64) -> Option<&SectionSummary> {
        self.sections
            .iter()
            .find(|section| section.contains_address(address))
    }

    pub fn file_offset_for_va(&self, address: u64) -> Option<u64> {
        self.section_containing_address(address)?
            .file_offset_for_va(address)
    }
}

impl SectionSummary {
    pub fn end_address(&self) -> u64 {
        self.address
            .saturating_add(self.virtual_size.max(self.file_size))
    }

    pub fn file_backed_end_address(&self) -> u64 {
        self.address.saturating_add(self.file_size)
    }

    pub fn contains_address(&self, address: u64) -> bool {
        address >= self.address && address < self.end_address()
    }

    pub fn file_offset_for_va(&self, address: u64) -> Option<u64> {
        if address < self.address || address >= self.file_backed_end_address() {
            return None;
        }

        self.file_offset
            .checked_add(address.checked_sub(self.address)?)
    }

    pub fn permissions(&self) -> String {
        format!(
            "{}{}{}",
            if self.readable { 'r' } else { '-' },
            if self.writable { 'w' } else { '-' },
            if self.executable { 'x' } else { '-' }
        )
    }

    pub fn is_string_candidate(&self) -> bool {
        self.file_size > 0 && !self.executable && (self.readable || self.writable)
    }

    pub fn is_executable_code_candidate(&self) -> bool {
        self.file_size > 0 && self.executable
    }
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

pub fn executable_sections<'a>(
    data: &'a [u8],
    analysis: &BinaryAnalysis,
) -> Vec<LoadedSection<'a>> {
    analysis
        .sections
        .iter()
        .filter(|section| section.is_executable_code_candidate())
        .filter_map(|section| load_section(data, section).ok())
        .collect()
}

pub fn load_section<'a>(data: &'a [u8], section: &SectionSummary) -> Result<LoadedSection<'a>> {
    let file_offset = usize::try_from(section.file_offset)
        .map_err(|_| anyhow::anyhow!("{} section file offset does not fit usize", section.name))?;
    let file_size = usize::try_from(section.file_size)
        .map_err(|_| anyhow::anyhow!("{} section file size does not fit usize", section.name))?;
    let bytes = checked_slice(data, file_offset, file_size, &section.name)?;

    Ok(LoadedSection {
        name: section.name.clone(),
        va: section.address,
        bytes,
    })
}

fn analyze_pe<'a>(data: &'a [u8], pe: goblin::pe::PE<'_>) -> Result<AnalyzedBinary<'a>> {
    let base = pe.image_base as u64;
    let machine = pe.header.coff_header.machine;
    let architecture = pe_architecture(machine)?;

    let analysis = analyze_pe_names(data, &pe, base);
    let section = analysis
        .sections
        .iter()
        .find(|section| section.name == ".text" && section.is_executable_code_candidate())
        .or_else(|| {
            analysis
                .sections
                .iter()
                .find(|section| section.is_executable_code_candidate())
        })
        .ok_or_else(|| anyhow::anyhow!("no executable section found"))?;

    let loaded = load_section(data, section)?;

    Ok(AnalyzedBinary {
        metadata: BinaryMetadata {
            format: BinaryFormat::PE,
            architecture,
            bitness: if pe.is_64 { 64 } else { 32 },
            endianness: Endianness::Little,
            entry_point: Some(base + pe.entry as u64),
        },
        text: TextSection {
            va: loaded.va,
            bytes: loaded.bytes,
        },
        analysis,
    })
}

fn analyze_elf<'a>(data: &'a [u8], elf: goblin::elf::Elf<'_>) -> Result<AnalyzedBinary<'a>> {
    let architecture = elf_architecture(elf.header.e_machine, elf.is_64)?;
    let analysis = analyze_elf_names(data, &elf);
    let section = analysis
        .sections
        .iter()
        .find(|section| section.name == ".text" && section.is_executable_code_candidate())
        .or_else(|| {
            analysis
                .sections
                .iter()
                .find(|section| section.is_executable_code_candidate())
        })
        .ok_or_else(|| anyhow::anyhow!("no executable section found"))?;

    let loaded = load_section(data, section)?;

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
            va: loaded.va,
            bytes: loaded.bytes,
        },
        analysis,
    })
}

fn analyze_pe_names(data: &[u8], pe: &goblin::pe::PE<'_>, base: u64) -> BinaryAnalysis {
    let imports: Vec<ImportSummary> = pe
        .imports
        .iter()
        .map(|import| ImportSummary {
            address: Some(base + import.offset as u64),
            library: Some(import.dll.to_string()),
            name: import.name.to_string(),
        })
        .collect();

    let exports: Vec<ExportSummary> = pe
        .exports
        .iter()
        .map(|export| {
            let address = base + export.rva as u64;
            ExportSummary {
                address: Some(address),
                name: export
                    .name
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("export_{address:x}")),
                kind: SymbolKind::Export,
                size: export.size,
                forwarder: export.reexport.as_ref().map(format_pe_reexport),
            }
        })
        .collect();

    let mut symbols: Vec<SymbolSummary> = exports
        .iter()
        .map(|export| SymbolSummary {
            address: export.address,
            name: export.name.clone(),
            kind: SymbolKind::Export,
        })
        .collect();

    symbols.extend(pe.imports.iter().map(|import| SymbolSummary {
        address: Some(base + import.offset as u64),
        name: format!("{}!{}", import.dll, import.name),
        kind: SymbolKind::Import,
    }));

    let sections = summarize_pe_sections(pe, base);
    let relocations = summarize_pe_relocations(data, pe, base, &sections);
    let function_ranges = summarize_pe_unwind_functions(pe, base, &sections);
    let strings = extract_section_strings(
        data,
        sections
            .iter()
            .filter(|section| section.is_string_candidate()),
    );
    let data_objects = extract_data_objects(
        data,
        &sections,
        &imports,
        &symbols,
        &strings,
        if pe.is_64 { 8 } else { 4 },
        Endianness::Little,
    );

    BinaryAnalysis {
        imports,
        exports,
        symbols,
        strings,
        data_objects,
        relocations,
        function_ranges,
        sections,
    }
}

fn analyze_elf_names(data: &[u8], elf: &goblin::elf::Elf<'_>) -> BinaryAnalysis {
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut symbols = Vec::new();

    collect_elf_symbols(
        elf.dynsyms.iter(),
        &elf.dynstrtab,
        &mut symbols,
        Some(&mut imports),
        Some(&mut exports),
    );
    collect_elf_symbols(elf.syms.iter(), &elf.strtab, &mut symbols, None, None);

    let sections = summarize_elf_sections(elf);
    let relocations = summarize_elf_relocations(elf, &sections);
    let strings = extract_section_strings(
        data,
        sections
            .iter()
            .filter(|section| section.is_string_candidate()),
    );
    let data_objects = extract_data_objects(
        data,
        &sections,
        &imports,
        &symbols,
        &strings,
        if elf.is_64 { 8 } else { 4 },
        if elf.little_endian {
            Endianness::Little
        } else {
            Endianness::Big
        },
    );

    BinaryAnalysis {
        imports,
        exports,
        symbols,
        strings,
        data_objects,
        relocations,
        function_ranges: Vec::new(),
        sections,
    }
}

fn summarize_pe_sections(pe: &goblin::pe::PE<'_>, base: u64) -> Vec<SectionSummary> {
    const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
    const IMAGE_SCN_MEM_READ: u32 = 0x4000_0000;
    const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;

    let mut sections: Vec<SectionSummary> = pe
        .sections
        .iter()
        .filter_map(|section| {
            let name = section.name().unwrap_or("").to_string();
            if name.is_empty() {
                return None;
            }

            let characteristics = section.characteristics;
            Some(SectionSummary {
                name,
                address: base + section.virtual_address as u64,
                virtual_size: section.virtual_size as u64,
                file_offset: section.pointer_to_raw_data as u64,
                file_size: section.size_of_raw_data as u64,
                readable: characteristics & IMAGE_SCN_MEM_READ != 0,
                writable: characteristics & IMAGE_SCN_MEM_WRITE != 0,
                executable: characteristics & IMAGE_SCN_MEM_EXECUTE != 0,
            })
        })
        .collect();

    sections.sort_by_key(|section| section.address);
    sections
}

fn summarize_elf_sections(elf: &goblin::elf::Elf<'_>) -> Vec<SectionSummary> {
    const SHF_WRITE: u64 = 0x1;
    const SHF_ALLOC: u64 = 0x2;
    const SHF_EXECINSTR: u64 = 0x4;

    let mut sections: Vec<SectionSummary> = elf
        .section_headers
        .iter()
        .filter_map(|section| {
            let name = elf.shdr_strtab.get_at(section.sh_name)?.to_string();
            if name.is_empty() {
                return None;
            }

            Some(SectionSummary {
                name,
                address: section.sh_addr,
                virtual_size: section.sh_size,
                file_offset: section.sh_offset,
                file_size: section.sh_size,
                readable: section.sh_flags & SHF_ALLOC != 0,
                writable: section.sh_flags & SHF_WRITE != 0,
                executable: section.sh_flags & SHF_EXECINSTR != 0,
            })
        })
        .collect();

    sections.sort_by_key(|section| section.address);
    sections
}

fn summarize_pe_unwind_functions(
    pe: &goblin::pe::PE<'_>,
    base: u64,
    sections: &[SectionSummary],
) -> Vec<FunctionRangeSummary> {
    let Some(exception_data) = &pe.exception_data else {
        return Vec::new();
    };

    let mut ranges = Vec::new();
    for runtime_function in exception_data.functions().flatten() {
        if runtime_function.begin_address == runtime_function.end_address {
            continue;
        }

        let Some(start) = base.checked_add(runtime_function.begin_address as u64) else {
            continue;
        };
        let Some(end) = base.checked_add(runtime_function.end_address as u64) else {
            continue;
        };
        if start >= end {
            continue;
        }

        let Some(section) = sections.iter().find(|section| {
            section.is_executable_code_candidate() && section.contains_address(start)
        }) else {
            continue;
        };
        if !section.contains_address(end.saturating_sub(1)) {
            continue;
        }

        ranges.push(FunctionRangeSummary {
            start,
            end,
            section: Some(section.name.clone()),
            source: "PE unwind".to_string(),
            unwind_info: base.checked_add(runtime_function.unwind_info_address as u64),
        });
    }

    ranges.sort_by_key(|range| (range.start, range.end));
    ranges.dedup_by_key(|range| (range.start, range.end, range.unwind_info));
    ranges
}

fn summarize_pe_relocations(
    data: &[u8],
    pe: &goblin::pe::PE<'_>,
    base: u64,
    sections: &[SectionSummary],
) -> Vec<RelocationSummary> {
    let mut relocations = Vec::new();

    if let Some(directory) = pe
        .header
        .optional_header
        .as_ref()
        .and_then(|header| header.data_directories.get_base_relocation_table())
    {
        if let Some(file_offset) =
            pe_rva_to_file_offset(directory.virtual_address as u64, base, sections)
        {
            let directory_size = directory.size as usize;
            let end = file_offset
                .checked_add(directory_size)
                .map(|end| end.min(data.len()))
                .unwrap_or(data.len());
            let mut cursor = file_offset;

            while cursor + 8 <= end && relocations.len() < MAX_RELOCATIONS {
                let block_start = cursor;
                let Some(page_rva) = read_u32_le(data, cursor) else {
                    break;
                };
                let Some(block_size) = read_u32_le(data, cursor + 4).map(|size| size as usize)
                else {
                    break;
                };
                if block_size < 8 {
                    break;
                }

                let Some(block_end) = block_start
                    .checked_add(block_size)
                    .map(|block_end| block_end.min(end))
                else {
                    break;
                };
                cursor = block_start + 8;

                while cursor + 2 <= block_end && relocations.len() < MAX_RELOCATIONS {
                    let Some(entry) = read_u16_le(data, cursor) else {
                        break;
                    };
                    let typ = entry >> 12;
                    let offset = entry & 0x0fff;
                    if typ != 0 {
                        let address = base + page_rva as u64 + offset as u64;
                        relocations.push(RelocationSummary {
                            address,
                            section: section_name_for_address(sections, address),
                            source: "PE base".to_string(),
                            kind: pe_base_relocation_type_name(typ).to_string(),
                            type_id: typ as u64,
                            symbol: None,
                            addend: None,
                        });
                    }
                    cursor += 2;
                }

                cursor = block_end;
            }
        }
    }

    for section in &pe.sections {
        let Ok(section_name) = section.name() else {
            continue;
        };
        let Ok(section_relocations) = section.relocations(data) else {
            continue;
        };
        for relocation in
            section_relocations.take(MAX_RELOCATIONS.saturating_sub(relocations.len()))
        {
            let address = base + relocation.virtual_address as u64;
            relocations.push(RelocationSummary {
                address,
                section: Some(section_name.to_string()),
                source: "PE COFF".to_string(),
                kind: pe_coff_relocation_type_name(pe.header.coff_header.machine, relocation.typ)
                    .to_string(),
                type_id: relocation.typ as u64,
                symbol: Some(format!("symbol#{}", relocation.symbol_table_index)),
                addend: None,
            });
        }
    }

    relocations.sort_by_key(|relocation| relocation.address);
    relocations
}

fn summarize_elf_relocations(
    elf: &goblin::elf::Elf<'_>,
    sections: &[SectionSummary],
) -> Vec<RelocationSummary> {
    let mut relocations = Vec::new();
    collect_elf_relocations(
        elf.dynrelas.iter(),
        "ELF dynamic rela",
        elf.header.e_machine,
        &elf.dynstrtab,
        &elf.dynsyms,
        sections,
        &mut relocations,
    );
    collect_elf_relocations(
        elf.dynrels.iter(),
        "ELF dynamic rel",
        elf.header.e_machine,
        &elf.dynstrtab,
        &elf.dynsyms,
        sections,
        &mut relocations,
    );
    collect_elf_relocations(
        elf.pltrelocs.iter(),
        "ELF PLT",
        elf.header.e_machine,
        &elf.dynstrtab,
        &elf.dynsyms,
        sections,
        &mut relocations,
    );

    for (section_idx, section_relocations) in &elf.shdr_relocs {
        let source = elf
            .section_headers
            .get(*section_idx)
            .and_then(|section| elf.shdr_strtab.get_at(section.sh_name))
            .map(|name| format!("ELF section {name}"))
            .unwrap_or_else(|| format!("ELF section #{section_idx}"));
        collect_elf_relocations(
            section_relocations.iter(),
            &source,
            elf.header.e_machine,
            &elf.strtab,
            &elf.syms,
            sections,
            &mut relocations,
        );
        if relocations.len() >= MAX_RELOCATIONS {
            break;
        }
    }

    relocations.sort_by_key(|relocation| relocation.address);
    relocations
}

fn collect_elf_relocations<'a>(
    relocation_iter: impl Iterator<Item = goblin::elf::Reloc> + 'a,
    source: &str,
    machine: u16,
    names: &goblin::strtab::Strtab<'_>,
    symbols: &goblin::elf::sym::Symtab<'_>,
    sections: &[SectionSummary],
    relocations: &mut Vec<RelocationSummary>,
) {
    for relocation in relocation_iter {
        if relocations.len() >= MAX_RELOCATIONS {
            break;
        }
        let symbol = symbols
            .get(relocation.r_sym)
            .and_then(|symbol| names.get_at(symbol.st_name))
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        relocations.push(RelocationSummary {
            address: relocation.r_offset,
            section: section_name_for_address(sections, relocation.r_offset),
            source: source.to_string(),
            kind: reloc::r_to_str(relocation.r_type, machine).to_string(),
            type_id: relocation.r_type as u64,
            symbol,
            addend: relocation.r_addend,
        });
    }
}

fn pe_rva_to_file_offset(rva: u64, base: u64, sections: &[SectionSummary]) -> Option<usize> {
    let address = base.checked_add(rva)?;
    let file_offset = sections
        .iter()
        .find_map(|section| section.file_offset_for_va(address))?;
    usize::try_from(file_offset).ok()
}

fn section_name_for_address(sections: &[SectionSummary], address: u64) -> Option<String> {
    sections
        .iter()
        .find(|section| section.contains_address(address))
        .map(|section| section.name.clone())
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn pe_base_relocation_type_name(typ: u16) -> &'static str {
    match typ {
        0 => "ABSOLUTE",
        1 => "HIGH",
        2 => "LOW",
        3 => "HIGHLOW",
        4 => "HIGHADJ",
        5 => "MIPS_JMPADDR/ARM_MOV32/RISCV_HIGH20",
        7 => "THUMB_MOV32/RISCV_LOW12I",
        8 => "RISCV_LOW12S",
        9 => "MIPS_JMPADDR16",
        10 => "DIR64",
        _ => "UNKNOWN",
    }
}

fn pe_coff_relocation_type_name(machine: u16, typ: u16) -> &'static str {
    match machine {
        COFF_MACHINE_X86 => match typ {
            0x0000 => "ABSOLUTE",
            0x0006 => "DIR32",
            0x0007 => "DIR32NB",
            0x000a => "SECTION",
            0x000b => "SECREL",
            0x0014 => "REL32",
            _ => "UNKNOWN",
        },
        COFF_MACHINE_X86_64 => match typ {
            0x0000 => "ABSOLUTE",
            0x0001 => "ADDR64",
            0x0002 => "ADDR32",
            0x0003 => "ADDR32NB",
            0x0004 => "REL32",
            0x0005 => "REL32_1",
            0x0006 => "REL32_2",
            0x0007 => "REL32_3",
            0x0008 => "REL32_4",
            0x0009 => "REL32_5",
            0x000a => "SECTION",
            0x000b => "SECREL",
            0x000c => "SECREL7",
            0x000d => "TOKEN",
            0x000e => "SREL32",
            0x000f => "PAIR",
            0x0010 => "SSPAN32",
            _ => "UNKNOWN",
        },
        _ => "UNKNOWN",
    }
}
fn collect_elf_symbols<'a>(
    symbols_iter: impl Iterator<Item = goblin::elf::sym::Sym> + 'a,
    names: &goblin::strtab::Strtab<'_>,
    symbols: &mut Vec<SymbolSummary>,
    mut imports: Option<&mut Vec<ImportSummary>>,
    mut exports: Option<&mut Vec<ExportSummary>>,
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
        } else if is_elf_export_symbol(&symbol) {
            if let Some(exports) = exports.as_deref_mut() {
                exports.push(ExportSummary {
                    address: Some(symbol.st_value),
                    name: name.to_string(),
                    kind: kind.clone(),
                    size: symbol_size_to_usize(symbol.st_size),
                    forwarder: None,
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

fn format_pe_reexport(reexport: &Reexport<'_>) -> String {
    match reexport {
        Reexport::DLLName { export, lib } => format!("{lib}.{export}"),
        Reexport::DLLOrdinal { ordinal, lib } => format!("{lib}.#{ordinal}"),
    }
}

fn is_elf_export_symbol(symbol: &sym::Sym) -> bool {
    if symbol.st_value == 0 {
        return false;
    }

    let bind = sym::st_bind(symbol.st_info);
    if !matches!(bind, STB_GLOBAL | STB_WEAK) {
        return false;
    }

    matches!(
        sym::st_visibility(symbol.st_other),
        STV_DEFAULT | STV_PROTECTED
    )
}

fn symbol_size_to_usize(size: u64) -> usize {
    usize::try_from(size).unwrap_or(usize::MAX)
}

fn extract_data_objects(
    data: &[u8],
    sections: &[SectionSummary],
    imports: &[ImportSummary],
    symbols: &[SymbolSummary],
    strings: &[StringSummary],
    pointer_size: usize,
    endianness: Endianness,
) -> Vec<DataObjectSummary> {
    const MAX_DATA_OBJECTS: usize = 10_000;

    if !matches!(pointer_size, 4 | 8) {
        return Vec::new();
    }

    let mut objects = Vec::new();
    for section in sections
        .iter()
        .filter(|section| section.is_string_candidate())
    {
        let Ok(file_offset) = usize::try_from(section.file_offset) else {
            continue;
        };
        let Ok(size) = usize::try_from(section.file_size) else {
            continue;
        };
        let Some(end) = file_offset.checked_add(size) else {
            continue;
        };
        if file_offset >= data.len() || end > data.len() {
            continue;
        }

        let bytes = &data[file_offset..end];
        let aligned_len = bytes.len().saturating_sub(pointer_size) + 1;
        for offset in (0..aligned_len).step_by(pointer_size) {
            let Some(value) = read_pointer_value(&bytes[offset..offset + pointer_size], endianness)
            else {
                continue;
            };
            if value == 0 {
                continue;
            }

            let Some(target_section) = sections
                .iter()
                .find(|candidate| candidate.contains_address(value))
            else {
                continue;
            };

            let address = section.address.saturating_add(offset as u64);
            let target_label = describe_data_target(value, imports, symbols, strings);
            objects.push(DataObjectSummary {
                address,
                section: Some(section.name.clone()),
                kind: DataObjectKind::Pointer,
                size: pointer_size,
                value,
                target: value,
                target_section: Some(target_section.name.clone()),
                target_label,
            });

            if objects.len() >= MAX_DATA_OBJECTS {
                objects.sort_by_key(|object| (object.address, object.target));
                return objects;
            }
        }
    }

    objects.sort_by_key(|object| (object.address, object.target));
    objects
}

fn read_pointer_value(bytes: &[u8], endianness: Endianness) -> Option<u64> {
    match bytes.len() {
        4 => {
            let raw = <[u8; 4]>::try_from(bytes).ok()?;
            Some(match endianness {
                Endianness::Little => u32::from_le_bytes(raw) as u64,
                Endianness::Big => u32::from_be_bytes(raw) as u64,
            })
        }
        8 => {
            let raw = <[u8; 8]>::try_from(bytes).ok()?;
            Some(match endianness {
                Endianness::Little => u64::from_le_bytes(raw),
                Endianness::Big => u64::from_be_bytes(raw),
            })
        }
        _ => None,
    }
}

fn describe_data_target(
    address: u64,
    imports: &[ImportSummary],
    symbols: &[SymbolSummary],
    strings: &[StringSummary],
) -> Option<String> {
    if let Some(import) = imports
        .iter()
        .find(|import| import.address == Some(address))
    {
        let library = import.library.as_deref().unwrap_or("unknown");
        return Some(format!("{library}!{}", import.name));
    }

    if let Some(symbol) = symbols
        .iter()
        .find(|symbol| symbol.address == Some(address))
    {
        return Some(symbol.name.clone());
    }

    strings
        .iter()
        .find(|string| {
            address >= string.address
                && address
                    < string
                        .address
                        .saturating_add((string.value.len() as u64).max(1))
        })
        .map(|string| truncate_for_display(&string.value, 72))
}

fn extract_section_strings<'a>(
    data: &[u8],
    sections: impl Iterator<Item = &'a SectionSummary>,
) -> Vec<StringSummary> {
    let mut strings = Vec::new();

    for section in sections {
        let Ok(file_offset) = usize::try_from(section.file_offset) else {
            continue;
        };
        let Ok(size) = usize::try_from(section.file_size) else {
            continue;
        };
        let Some(end) = file_offset.checked_add(size) else {
            continue;
        };
        if file_offset >= data.len() || end > data.len() {
            continue;
        }

        strings.extend(extract_printable_strings_with_section(
            section.address,
            &data[file_offset..end],
            4,
            Some(section.name.as_str()),
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
    extract_printable_strings_with_section(base_address, bytes, min_len, None)
}

fn extract_printable_strings_with_section(
    base_address: u64,
    bytes: &[u8],
    min_len: usize,
    section: Option<&str>,
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
                section: section.map(str::to_string),
                value: String::from_utf8_lossy(&current).to_string(),
            });
        }
        current.clear();
    }

    if current.len() >= min_len {
        strings.push(StringSummary {
            address: base_address + start as u64,
            section: section.map(str::to_string),
            value: String::from_utf8_lossy(&current).to_string(),
        });
    }

    strings
}

fn is_printable_ascii(byte: u8) -> bool {
    matches!(byte, b'\t' | b' '..=b'~')
}

fn truncate_for_display(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        truncated.push_str("...");
    }
    truncated
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
                    section: None,
                    value: "hello world".to_string()
                },
                StringSummary {
                    address: 0x1011,
                    section: None,
                    value: "rust".to_string()
                },
                StringSummary {
                    address: 0x1016,
                    section: None,
                    value: "tooling".to_string()
                }
            ]
        );
    }

    #[test]
    fn extracts_section_strings_only_from_data_like_sections() {
        let data = b"CODETEXT\0hello data\0";
        let sections = [
            SectionSummary {
                name: ".text".to_string(),
                address: 0x1000,
                virtual_size: 0x09,
                file_offset: 0,
                file_size: 0x09,
                readable: true,
                writable: false,
                executable: true,
            },
            SectionSummary {
                name: ".rdata".to_string(),
                address: 0x2000,
                virtual_size: 0x0b,
                file_offset: 0x09,
                file_size: 0x0b,
                readable: true,
                writable: false,
                executable: false,
            },
        ];

        let strings = extract_section_strings(
            data,
            sections
                .iter()
                .filter(|section| section.is_string_candidate()),
        );

        assert_eq!(
            strings,
            vec![StringSummary {
                address: 0x2000,
                section: Some(".rdata".to_string()),
                value: "hello data".to_string()
            }]
        );
    }

    #[test]
    fn extracts_data_pointers_from_data_like_sections() {
        let mut data = vec![0u8; 0x20];
        data[0x10..0x18].copy_from_slice(&0x1000u64.to_le_bytes());
        let sections = vec![
            SectionSummary {
                name: ".text".to_string(),
                address: 0x1000,
                virtual_size: 0x10,
                file_offset: 0,
                file_size: 0x10,
                readable: true,
                writable: false,
                executable: true,
            },
            SectionSummary {
                name: ".rdata".to_string(),
                address: 0x3000,
                virtual_size: 0x10,
                file_offset: 0x10,
                file_size: 0x10,
                readable: true,
                writable: false,
                executable: false,
            },
        ];
        let symbols = vec![SymbolSummary {
            address: Some(0x1000),
            name: "entry".to_string(),
            kind: SymbolKind::Function,
        }];

        let objects =
            extract_data_objects(&data, &sections, &[], &symbols, &[], 8, Endianness::Little);

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].address, 0x3000);
        assert_eq!(objects[0].section.as_deref(), Some(".rdata"));
        assert_eq!(objects[0].kind, DataObjectKind::Pointer);
        assert_eq!(objects[0].size, 8);
        assert_eq!(objects[0].value, 0x1000);
        assert_eq!(objects[0].target, 0x1000);
        assert_eq!(objects[0].target_section.as_deref(), Some(".text"));
        assert_eq!(objects[0].target_label.as_deref(), Some("entry"));
    }
    #[test]
    fn analyzes_pe_imports_symbols_and_strings() {
        let data = include_bytes!("../tests/notepad.exe");

        let analyzed = analyze_binary(data).expect("test PE should parse");

        assert_eq!(analyzed.metadata.format, BinaryFormat::PE);
        assert!(!analyzed.analysis.imports.is_empty());
        assert!(!analyzed.analysis.symbols.is_empty());
        assert!(!analyzed.analysis.strings.is_empty());
        assert!(!analyzed.analysis.relocations.is_empty());
        assert!(!analyzed.analysis.function_ranges.is_empty());
        assert!(analyzed
            .analysis
            .relocations
            .iter()
            .any(|relocation| relocation.source == "PE base"));
        assert!(analyzed
            .analysis
            .function_ranges
            .iter()
            .any(|function| function.source == "PE unwind"));
        assert!(analyzed.analysis.function_ranges.iter().all(|function| {
            function.start < function.end
                && analyzed
                    .analysis
                    .section_containing_address(function.start)
                    .is_some_and(|section| section.executable)
        }));
        assert!(analyzed
            .analysis
            .strings
            .iter()
            .all(|string| string.section.as_deref() != Some(".text")));
        let text_section = analyzed
            .analysis
            .sections
            .iter()
            .find(|section| section.name == ".text")
            .expect("PE fixture should have .text");
        assert!(text_section.executable);
        assert_eq!(
            analyzed.analysis.file_offset_for_va(text_section.address),
            Some(text_section.file_offset)
        );
        assert!(analyzed
            .analysis
            .imports
            .iter()
            .any(|import| import.library.is_some() && !import.name.is_empty()));
    }
}
