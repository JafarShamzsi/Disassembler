#[cfg(test)]
mod tests {
    use main::parser::get_text_section;
    use std::fs;

    #[test]
    fn finds_text_section() {
        let data = fs::read("tests/notepad.exe").unwrap();
        let sec = get_text_section(&data).unwrap();
        assert!(sec.bytes.len() > 0, ".text should not be empty");
    }
}
