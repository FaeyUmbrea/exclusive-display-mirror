use std::fs;
use std::path::Path;

pub fn move_all_up_one_level(from: &Path, to: &Path) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(from).min_depth(1) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(from)?;
        let dest = to.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(entry.path(), &dest)?;
        }
    }
    Ok(())
}
