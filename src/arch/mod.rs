// for the whole architecture traits and common things

use std::fmt;

use crate::arch::x86::Instruction;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Architecture {
    X86,
    X64,
    ARM,
    AArch64,
    RISCV32,
    RISCV64,
    MIPS,
    MIPS64,
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86 => "x86",
            Architecture::X64 => "x86_64",
            Architecture::ARM => "arm",
            Architecture::AArch64 => "aarch64",
            Architecture::RISCV32 => "riscv32",
            Architecture::RISCV64 => "riscv64",
            Architecture::MIPS => "mips",
            Architecture::MIPS64 => "mips64",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endianness {
    Little,
    Big,
}

impl Endianness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Endianness::Little => "little",
            Endianness::Big => "big",
        }
    }
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ArchConfig {
    pub arch: Architecture,
    pub bitness: u32,
    pub endianness: Endianness,
    pub base_address: u64,
}

// Import the DisasmOpts type from x86legacy (our main implementation)
pub use x86::DisasmOpts;

pub trait ArchDisassembler {
    fn disassemble(&self, bytes: &[u8], config: &ArchConfig) -> Vec<Instruction>;
    fn supported_architectures(&self) -> &'static [Architecture];
    fn can_handle(&self, arch: Architecture) -> bool;
    fn detect_calls(&self, instruction: &str) -> bool;
    fn detect_jumps(&self, instruction: &str) -> bool;
    fn detect_returns(&self, instruction: &str) -> bool;
}

// Module declarations
pub mod arm;
pub mod x86; // This is our main x86 implementation // Placeholder for future
