use iced_x86::{Code, Decoder, DecoderOptions, Encoder, Instruction as IcedInstruction, Register};
use std::fmt;

#[derive(Debug, Clone)]
pub struct DisasmOpts {
    pub base_address: u64,
    pub bitness: u32,
}

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

pub fn disasm(bytes: &[u8], opts: DisasmOpts) -> Vec<Instruction> {
    let mut decoder = Decoder::with_ip(
        opts.bitness.try_into().unwrap(),
        bytes,
        opts.base_address,
        DecoderOptions::NONE,
    );
    let mut instructions = Vec::new();

    while decoder.can_decode() {
        let instr = decoder.decode();
        let mut bytes_buf = bytes[offset..offset + size].to_vec();

        let ip = instr.ip();
        let offset = (ip - opts.base_address) as usize;
        let size = instr.len() as usize;
        let raw_bytes = bytes[offset..offset + size].to_vec();

        let mut output = String::new();
        let formatter = iced_x86::NasmFormatter::new();
        let mut formatter_output = iced_x86::FormatterOutputString::new(&mut output);
        formatter.format(&instr, &mut formatter_output);

        instructions.push(Instruction {
            address: instr.ip(),
            bytes: bytes_buf,
            text: output,
        });
    }

    instructions
}
