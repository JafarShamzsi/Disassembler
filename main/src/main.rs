use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

use clap::Parser;

pub mod disassembler;
<<<<<<< HEAD
// Commented out until you create these modules
// pub mod exe;
// pub mod util;
// pub mod x86;

use disassembler::{DisasmOpts, disasm};
=======
pub mod parser;

use disassembler::{DisasmOpts, Instruction, disasm};
use parser::TextSection;
>>>>>>> 62f665544760dc494dd6518dd010b42ff3c0a23b

#[derive(Debug, Clone, Parser)]
pub struct Opts {
    #[clap(long)]
    raw: bool,

    #[clap(long)]
    cfg: bool,

    #[clap(name = "FILE", value_parser)]
    files: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    env_logger::init();

    let opts = Opts::parse();
    if opts.files.is_empty() {
        eprintln!("No files provided");
        std::process::exit(1);
    }

    for path in &opts.files {
        let data = {
            let mut f = BufReader::new(File::open(path)?);
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            buf
        };

        let TextSection { va, bytes } = parser::get_text_section(&data).unwrap_or_else(|e| {
            eprintln!("{}: failed to parse .text: {}", path.display(), e);
            std::process::exit(1);
        });

<<<<<<< HEAD
        println!("Successfully loaded text section at VA {:#x} with {} bytes", va, bytes.len());

        // Disassemble the loaded bytes
        let disasm_opts = DisasmOpts {
            base_address: va,
            bitness: 32, // Assuming 32-bit, adjust if needed
        };

        let instructions = disasm(bytes, disasm_opts);
        for inst in instructions {
            println!("{}", inst);
        }
    }

    Ok(())
}
=======
        println!(
            "Successfully loaded text section from {} at VA {:#x} with {} bytes",
            path.display(),
            va,
            bytes.len()
        );

        let disasm_opts = DisasmOpts {
            base_address: va,
            bitness: 64,
        };

        let instructions = disasm(&bytes, disasm_opts);
        for inst in instructions {
            println!("{}", inst); // uses fmt::Display
        }

        println!(); // spacing between files
    }

    Ok(())
}
>>>>>>> 62f665544760dc494dd6518dd010b42ff3c0a23b
