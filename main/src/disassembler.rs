<<<<<<< HEAD
use iced_x86::{Decoder, DecoderOptions, NasmFormatter, Formatter, FormatterOutput};
=======
use iced_x86::{Code, Decoder, DecoderOptions, Encoder, Instruction as IcedInstruction, Register};
>>>>>>> 62f665544760dc494dd6518dd010b42ff3c0a23b
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
        opts.bitness,
        bytes,
        opts.base_address,
        DecoderOptions::NONE,
    );

    let mut instructions = Vec::new();

    while decoder.can_decode() {
        let instr = decoder.decode();
<<<<<<< HEAD
=======
        let mut bytes_buf = bytes[offset..offset + size].to_vec();
>>>>>>> 62f665544760dc494dd6518dd010b42ff3c0a23b

        let ip = instr.ip();
        let offset = (ip - opts.base_address) as usize;
        let size = instr.len() as usize;
<<<<<<< HEAD
        if offset + size > bytes.len() {
            break;
        }

=======
>>>>>>> 62f665544760dc494dd6518dd010b42ff3c0a23b
        let raw_bytes = bytes[offset..offset + size].to_vec();

        let mut output = String::new();
        let formatter = NasmFormatter::new();
        let mut formatter_output = FormatterOutput::new(&mut output); 
        

        instructions.push(Instruction {
            address: ip,
            bytes: raw_bytes,
            text: output,
        });
    }

    instructions
}
