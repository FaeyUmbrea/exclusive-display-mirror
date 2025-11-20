use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
mod common;
mod test_utils;
use test_utils::write_manifest;

#[test]
fn manifest_cleanup_idempotency() {
    let temp = tempdir().unwrap();
    let file1 = temp.path().join("foo.txt");
    let file2 = temp.path().join("bar/baz.txt");
    fs::create_dir_all(file2.parent().unwrap()).unwrap();
    fs::write(&file1, b"test").unwrap();
    fs::write(&file2, b"test").unwrap();
    write_manifest(temp.path(), &["foo.txt", "bar/baz.txt"], "1.2.3");
    let exe = assert_cmd::cargo::cargo_bin("bootstrapper");
    let mut cmd = Command::new(&exe);
    cmd.current_dir(temp.path());
    common::set_test_mode(&mut cmd);
    cmd.arg("--cleanup");
    cmd.assert().success();
    // Run again, should print nothing to clean
    let mut cmd2 = Command::new(&exe);
    cmd2.current_dir(temp.path());
    common::set_test_mode(&mut cmd2);
    cmd2.arg("--cleanup");
    cmd2.assert()
        .success()
        .stdout(predicates::str::contains("nothing to clean"));
}
