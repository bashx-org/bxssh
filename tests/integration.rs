use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.arg("--help");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A WebAssembly-compatible SSH client CLI"))
        .stdout(predicate::str::contains("Usage: bxssh"))
        .stdout(predicate::str::contains("--user"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.arg("--version");
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("bxssh 0.1.0"));
}

#[test]
fn test_cli_missing_required_args() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.arg("localhost");
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_cli_invalid_port() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&["--user", "testuser", "--port", "invalid", "localhost"]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid port number"));
}

#[test]
fn test_cli_port_out_of_range() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&["--user", "testuser", "--port", "70000", "localhost"]);
    
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid port number"));
}

#[test]
fn test_cli_with_valid_args_but_connection_fails() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&[
        "--user", "testuser", 
        "--identity", "/nonexistent/key",
        "nonexistent-host.local"
    ]);
    
    // This should fail at the connection stage, not argument parsing
    cmd.assert()
        .failure();
}

#[test]
fn test_cli_with_command_option() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&[
        "--user", "testuser",
        "--command", "echo hello",
        "--identity", "/nonexistent/key", 
        "nonexistent-host.local"
    ]);
    
    // Should fail at connection, not argument parsing
    cmd.assert()
        .failure();
}

#[test]
fn test_cli_with_custom_port() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&[
        "--user", "testuser",
        "--port", "2222",
        "--identity", "/nonexistent/key",
        "nonexistent-host.local"
    ]);
    
    // Should fail at connection, not argument parsing
    cmd.assert()
        .failure();
}

#[test]
fn test_cli_empty_username() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&["--user", "", "localhost"]);
    
    cmd.assert()
        .failure();
}

#[test]
fn test_cli_empty_host() {
    let mut cmd = Command::cargo_bin("bxssh").unwrap();
    cmd.args(&["--user", "testuser", ""]);
    
    // This tests empty hostname handling
    cmd.assert()
        .failure();
}