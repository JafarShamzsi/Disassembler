use anyhow::{bail, Result};
use goblin::Object;

pub struct TextSection<'a> {
    pub va: u64,
    pub bytes: &'a [u8],
}

pub fn get_text_section(data: &[u8]) -> Result<TextSection<'_>> {
    match Object::parse(data)? {
        Object::PE(pe) => {
            let base = pe.image_base as u64;

            let section = pe
                .sections
                .iter()
                .find(|sect| sect.name().unwrap_or("") == ".text")
                .ok_or_else(|| anyhow::anyhow!(".text section not found"))?;

            let va = base + section.virtual_address as u64;
            let start = section.pointer_to_raw_data as usize;
            let len = section.size_of_raw_data as usize;
            let end = start
                .checked_add(len)
                .ok_or_else(|| anyhow::anyhow!(".text section range overflows usize"))?;

            if start >= data.len() || end > data.len() {
                bail!(
                    ".text section range is outside the file: start={}, size={}, file_size={}",
                    start,
                    len,
                    data.len()
                );
            }

            let bytes = &data[start..end];

            Ok(TextSection { va, bytes })
        }
        _ => bail!("not a PE file"),
    }
}
