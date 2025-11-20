use sevenz_rust2::decompress_file;
use std::path::Path;

pub fn extract_7z(archive: &Path, out_dir: &Path) -> anyhow::Result<()> {
    decompress_file(archive, out_dir)?;
    Ok(())
}
