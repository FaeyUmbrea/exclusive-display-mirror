use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;
mod common;

#[test]
fn cleanup_no_manifest_prints_nothing_to_clean() {
    let temp = tempdir().unwrap();
    let exe = assert_cmd::cargo::cargo_bin("bootstrapper");
    let mut cmd = Command::new(exe);
    cmd.current_dir(temp.path());
    common::set_test_mode(&mut cmd);
    cmd.arg("--cleanup");
    cmd.assert().success().stdout(predicates::str::contains(
        "No manifest found, nothing to clean.",
    ));
}

#[test]
fn cleanup_with_manifest_removes_files_and_manifest() {
    let temp = tempdir().unwrap();
    let manifest_path = temp.path().join("bootstrap_manifest.json");
    let file1 = temp.path().join("foo.txt");
    let file2 = temp.path().join("bar/baz.txt");
    fs::create_dir_all(file2.parent().unwrap()).unwrap();
    fs::write(&file1, b"test").unwrap();
    fs::write(&file2, b"test").unwrap();
    let manifest = serde_json::json!({
        "version": "1.2.3",
        "files": ["foo.txt", "bar/baz.txt"]
    });
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let exe = assert_cmd::cargo::cargo_bin("bootstrapper");
    let mut cmd = Command::new(exe);
    cmd.current_dir(temp.path());
    common::set_test_mode(&mut cmd);
    cmd.arg("--cleanup");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Cleanup complete."));
    assert!(!manifest_path.exists());
    assert!(!file1.exists());
    assert!(!file2.exists());
}
