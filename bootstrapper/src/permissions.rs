use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn is_writable(dir: &Path) -> bool {
    let test_path = dir.join(".perm_test");
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&test_path)
    {
        Ok(mut f) => {
            let _ = f.write_all(b"test");
            let _ = fs::remove_file(&test_path);
            true
        }
        Err(_) => false,
    }
}
