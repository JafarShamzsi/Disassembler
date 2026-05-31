use disassembler::arch::arm::ARMDisassembler;
use disassembler::arch::{ArchConfig, ArchDisassembler, Architecture, Endianness};

#[test]
fn arm_disassembler_handles_basic_arm_instruction() {
    let disassembler = ARMDisassembler;
    let config = ArchConfig {
        arch: Architecture::ARM,
        bitness: 32,
        endianness: Endianness::Little,
        base_address: 0x1000,
    };

    let instructions = disassembler.disassemble(&[0x1e, 0xff, 0x2f, 0xe1], &config);

    assert!(!instructions.is_empty());
    assert_eq!(instructions[0].address, 0x1000);
}
