use anyhow::{bail, Result};
use goblin::elf::header::{EM_386, EM_AARCH64, EM_ARM, EM_MIPS, EM_RISCV, EM_X86_64};
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
    })
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
