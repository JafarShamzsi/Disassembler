use super::{ArchConfig, ArchDisassembler, Architecture, Instruction};
use capstone::prelude::*;

pub struct ARMDisassembler;

impl ArchDisassembler for ARMDisassembler {
    fn disassemble(&self, bytes: &[u8], config: &ArchConfig) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Create capstone engine based on architecture
        let cs = match config.arch {
            Architecture::ARM => Capstone::new()
                .arm()
                .mode(arch::arm::ArchMode::Arm)
                .detail(true)
                .build(),
            Architecture::AArch64 => Capstone::new()
                .arm64()
                .mode(arch::arm64::ArchMode::Arm)
                .detail(true)
                .build(),
            _ => return instructions, // Unsupported architecture
        };

        let cs = match cs {
            Ok(cs) => cs,
            Err(_) => return instructions, // Failed to create engine
        };

        // Disassemble all bytes
        let insns = match cs.disasm_all(bytes, config.base_address) {
            Ok(insns) => insns,
            Err(_) => return instructions, // Failed to disassemble
        };

        // Convert capstone instructions to our Instruction format
        for insn in insns.as_ref() {
            let mnemonic = insn.mnemonic().unwrap_or("");
            let op_str = insn.op_str().unwrap_or("");

            let text = if op_str.is_empty() {
                mnemonic.to_string()
            } else {
                format!("{} {}", mnemonic, op_str)
            };

            instructions.push(Instruction {
                address: insn.address(),
                bytes: insn.bytes().to_vec(),
                text,
            });
        }

        instructions
    }

    fn supported_architectures(&self) -> &'static [Architecture] {
        &[Architecture::ARM, Architecture::AArch64]
    }

    fn can_handle(&self, arch: Architecture) -> bool {
        matches!(arch, Architecture::ARM | Architecture::AArch64)
    }

    fn detect_calls(&self, instruction: &str) -> bool {
        let mnemonic = instruction.split_whitespace().next().unwrap_or("");
        matches!(mnemonic, "bl" | "blx" | "blr")
    }

    fn detect_jumps(&self, instruction: &str) -> bool {
        let mnemonic = instruction.split_whitespace().next().unwrap_or("");
        matches!(
            mnemonic,
            "b" | "b.eq"
                | "b.ne"
                | "b.cs"
                | "b.hs"
                | "b.cc"
                | "b.lo"
                | "b.mi"
                | "b.pl"
                | "b.vs"
                | "b.vc"
                | "b.hi"
                | "b.ls"
                | "b.ge"
                | "b.lt"
                | "b.gt"
                | "b.le"
                | "b.al"
                | "br"
                | "beq"
                | "bne"
                | "bcs"
                | "bhs"
                | "bcc"
                | "blo"
                | "bmi"
                | "bpl"
                | "bvs"
                | "bvc"
                | "bhi"
                | "bls"
                | "bge"
                | "blt"
                | "bgt"
                | "ble"
                | "bal"
        )
    }

    fn detect_returns(&self, instruction: &str) -> bool {
        let instruction_lower = instruction.to_lowercase();
        instruction_lower.contains("ret")
            || (instruction_lower.contains("lr") && instruction_lower.starts_with("bx"))
            || instruction_lower.starts_with("mov pc")
            || instruction_lower == "ret"
    }
}
