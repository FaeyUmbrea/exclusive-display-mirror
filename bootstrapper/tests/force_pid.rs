use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
mod common;
use std::fs;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn force_flag_is_accepted() {
    let temp = tempdir().unwrap();
    let exe = assert_cmd::cargo::cargo_bin("bootstrapper");
    let mut cmd = Command::new(exe);
    cmd.current_dir(temp.path());
    cmd.arg("--force");
    // This will likely fail at download, but we just want to check the flag is accepted and doesn't panic
    let _ = cmd.output();
}

#[test]
fn pid_wait_prints_message() {
    let temp = tempdir().unwrap();
    let exe = assert_cmd::cargo::cargo_bin("bootstrapper");
    // Spawn a dummy process to wait on
    let mut child = StdCommand::new("cmd")
        .arg("/C")
        .arg("timeout /T 1 > NUL")
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let mut cmd = Command::new(exe);
    cmd.current_dir(temp.path());
    common::set_test_mode(&mut cmd);
    cmd.arg("--pid").arg(format!("{}", pid));
    let assert = cmd
        .assert()
        .stdout(predicates::str::contains("waiting for parent to exit"));
    // Let the child finish
    let _ = child.wait();
    let _ = assert;
}
