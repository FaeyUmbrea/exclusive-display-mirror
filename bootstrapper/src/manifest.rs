use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct BootstrapManifest {
    pub version: String,
    pub files: Vec<String>,
}

pub fn remove_manifest_files(base: &Path, manifest: &BootstrapManifest) -> anyhow::Result<()> {
    for rel in &manifest.files {
        let path = base.join(rel);
        if path.exists() {
            if path.is_file() {
                fs::remove_file(&path)?;
            } else if path.is_dir() {
                fs::remove_dir_all(&path)?;
            }
        }
    }
    Ok(())
}

pub fn write_manifest(path: &Path, manifest: &BootstrapManifest) -> anyhow::Result<()> {
    let f = fs::File::create(path)?;
    serde_json::to_writer_pretty(f, manifest)?;
    Ok(())
}

pub fn get_all_files_relative(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)?
                .to_string_lossy()
                .replace("\\", "/");
            files.push(rel);
        }
    }
    Ok(files)
}
