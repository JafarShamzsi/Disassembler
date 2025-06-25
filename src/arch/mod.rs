// for the whole architecture traits and common things

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Endianness {
    Little,
    Big,
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
