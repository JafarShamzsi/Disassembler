use super::{ArchConfig, ArchDisassembler, Architecture, Instruction};

pub struct ARMDisassembler;

impl ArchDisassembler for ARMDisassembler {
    fn disassemble(&self, _bytes: &[u8], _config: &ArchConfig) -> Vec<Instruction> {
        // TODO: Implement using capstone-rs or similar
        // For now, return empty vector
        vec![]
    }

    fn supported_architectures(&self) -> &'static [Architecture] {
        &[Architecture::ARM, Architecture::AArch64]
    }

    fn can_handle(&self, arch: Architecture) -> bool {
        matches!(arch, Architecture::ARM | Architecture::AArch64)
    }

    fn detect_calls(&self, instruction: &str) -> bool {
        instruction.starts_with("bl") || instruction.starts_with("blx")
    }

    fn detect_jumps(&self, instruction: &str) -> bool {
        let mnemonic = instruction.split_whitespace().next().unwrap_or("");
        matches!(
            mnemonic,
            "b" | "beq"
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
        instruction.contains("lr")
            && (instruction.starts_with("bx") || instruction.starts_with("mov pc"))
    }
}
