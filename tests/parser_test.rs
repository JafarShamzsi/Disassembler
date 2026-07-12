#[cfg(test)]
mod tests {
    use disassembler::arch::Architecture;
    use disassembler::parser::{
        analyze_binary, executable_sections, get_text_section, BinaryFormat,
    };
    use std::fs;

    #[test]
    fn finds_text_section() {
        let data = fs::read("tests/notepad.exe").unwrap();
        let sec = get_text_section(&data).unwrap();
        assert!(!sec.bytes.is_empty(), ".text should not be empty");
    }

    #[test]
    fn lists_file_backed_executable_sections() {
        let data = fs::read("tests/notepad.exe").unwrap();
        let binary = analyze_binary(&data).unwrap();
        let sections = executable_sections(&data, &binary.analysis);

        assert!(
            !sections.is_empty(),
            "PE fixture should expose executable sections"
        );
        assert!(sections.iter().any(|section| section.name == ".text"));
        assert!(sections.iter().all(|section| !section.bytes.is_empty()));
        assert!(sections.iter().all(|section| binary
            .analysis
            .section_containing_address(section.va)
            .is_some()));
    }
    #[test]
    fn detects_pe_metadata() {
        let data = fs::read("tests/notepad.exe").unwrap();
        let binary = analyze_binary(&data).unwrap();

        assert_eq!(binary.metadata.format, BinaryFormat::PE);
        assert_eq!(binary.metadata.architecture, Architecture::X64);
        assert_eq!(binary.metadata.bitness, 64);
        assert!(binary.metadata.entry_point.is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_elf_metadata() {
        let data = fs::read(std::env::current_exe().unwrap()).unwrap();
        let binary = analyze_binary(&data).unwrap();

        assert_eq!(binary.metadata.format, BinaryFormat::ELF);
        assert!(matches!(
            binary.metadata.architecture,
            Architecture::X64 | Architecture::AArch64
        ));
        assert!(!binary.text.bytes.is_empty(), ".text should not be empty");
    }
}
