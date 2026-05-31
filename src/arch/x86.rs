use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Instruction {
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#08x}: {}", self.address, self.text)
    }
}

#[derive(Debug, Clone)]
pub struct DisasmOpts {
    pub base_address: u64,
    pub bitness: u32,
}

pub fn disasm(bytes: &[u8], opts: DisasmOpts) -> Vec<Instruction> {
    let mut decoder =
        Decoder::with_ip(opts.bitness, bytes, opts.base_address, DecoderOptions::NONE);
    let mut instructions = Vec::new();

    while decoder.can_decode() {
        let instr = decoder.decode();
        let ip = instr.ip();
        let offset = (ip - opts.base_address) as usize;
        let size = instr.len();

        if offset + size > bytes.len() {
            break;
        }

        let raw_bytes = bytes[offset..offset + size].to_vec();
        let mut output = String::new();
        let mut formatter = NasmFormatter::new();
        formatter.format(&instr, &mut output);

        instructions.push(Instruction {
            address: ip,
            bytes: raw_bytes,
            text: output,
        });
    }

    instructions
}

// Architecture trait implementation
use super::{ArchConfig, ArchDisassembler, Architecture};

pub struct X86Disassembler;

impl ArchDisassembler for X86Disassembler {
    fn disassemble(&self, bytes: &[u8], config: &ArchConfig) -> Vec<Instruction> {
        let opts = DisasmOpts {
            base_address: config.base_address,
            bitness: config.bitness,
        };
        disasm(bytes, opts)
    }

    fn supported_architectures(&self) -> &'static [Architecture] {
        &[Architecture::X86, Architecture::X64]
    }

    fn can_handle(&self, arch: Architecture) -> bool {
        matches!(arch, Architecture::X86 | Architecture::X64)
    }

    fn detect_calls(&self, instruction: &str) -> bool {
        instruction.starts_with("call")
    }

    fn detect_jumps(&self, instruction: &str) -> bool {
        let mnemonic = instruction.split_whitespace().next().unwrap_or("");
        matches!(
            mnemonic,
            "jmp"
                | "je"
                | "jne"
                | "jz"
                | "jnz"
                | "ja"
                | "jb"
                | "jae"
                | "jbe"
                | "jg"
                | "jl"
                | "jge"
                | "jle"
                | "jo"
                | "jno"
                | "js"
                | "jns"
                | "jp"
                | "jnp"
        )
    }

    fn detect_returns(&self, instruction: &str) -> bool {
        instruction.starts_with("ret")
    }
}
