use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

use clap::Parser;

pub mod parser;
// Commented out until you create these modules
// pub mod exe;  
// pub mod util;  
// pub mod x86;  
// pub mod disassembler;

// Comment out these imports until you have the modules
// use exe::ExeExecutable;
// use x86::X86Executable;
// use disassembler::{Diasmopts, Instruction};

#[derive(Debug, Clone, Parser)]
pub struct Opts {
    #[clap(long)]
    raw: bool,

    #[clap(long)]
    cfg: bool,

    #[clap(name = "FILE", value_parser)] // --flag doesn't work added long to work it also without flag it runs 
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

        let parser::TextSection { va, bytes } =
            parser::get_text_section(&data)
                .unwrap_or_else(|e| {
                    eprintln!("{}: failed to parse .text: {}", path.display(), e);
                    std::process::exit(1);
                });

        // Comment out disassembler code until you implement it
        println!("Successfully loaded text section at VA {:#x} with {} bytes", va, bytes.len());
        
        /*
        if opts.cfg {
            let graph = disassembler::build_cfg(bytes, va as u64);
            println!("{}", graph.to_dot());
        } else {
            let disasm_opts = DisasmOpts { raw: opts.raw };
            for inst in disassembler::disasm(bytes, va as u64, &disasm_opts) {
                if opts.raw {
                    println!("{:#08x}: {:02x?}", inst.address, inst.bytes);
                } else {
                    println!("{:#08x}: {}", inst.address, inst.text);
                }
            }
        }
        */
    }

    Ok(())
}