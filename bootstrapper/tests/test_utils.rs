use std::fs;
use std::path::Path;

pub fn write_manifest<P: AsRef<Path>>(dir: P, files: &[&str], version: &str) {
    let manifest = serde_json::json!({
        "version": version,
        "files": files
    });
    let manifest_path = dir.as_ref().join("bootstrap_manifest.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}
