use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn secret_agent() -> Command {
    Command::cargo_bin("secret-agent").unwrap()
}

/// Create a temporary vault directory for isolated tests
fn setup_test_env() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::env::set_var("SECRET_AGENT_VAULT_DIR", dir.path());
    std::env::set_var("SECRET_AGENT_USE_FILE", "1");
    dir
}

#[test]
#[serial]
fn test_exec_without_separator() {
    // exec should work without -- separator
    secret_agent()
        .args(["exec", "echo", "hello", "world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world"));
}

#[test]
#[serial]
fn test_exec_with_env_flag_no_separator() {
    let _dir = setup_test_env();

    // Create a test secret
    secret_agent()
        .args(["create", "TEST_EXEC_KEY", "--force"])
        .assert()
        .success();

    // exec --env should work without -- separator
    secret_agent()
        .args([
            "exec",
            "--env",
            "TEST_EXEC_KEY",
            "printenv",
            "TEST_EXEC_KEY",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:TEST_EXEC_KEY]"));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_EXEC_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_import_replace_flag() {
    let _dir = setup_test_env();

    // Create initial secret
    secret_agent()
        .args(["create", "TEST_REPLACE_KEY", "--force"])
        .assert()
        .success();

    // Import without --replace should fail
    secret_agent()
        .args(["import", "TEST_REPLACE_KEY"])
        .write_stdin("new_value\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Import with --replace should succeed
    secret_agent()
        .args(["import", "TEST_REPLACE_KEY", "--replace"])
        .write_stdin("new_value\n")
        .assert()
        .success();

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_REPLACE_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_inject_env_format() {
    let _dir = setup_test_env();
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    // Create a test secret
    secret_agent()
        .args(["create", "TEST_INJECT_KEY", "--force"])
        .assert()
        .success();

    // Inject with --env-format
    secret_agent()
        .args([
            "inject",
            "TEST_INJECT_KEY",
            "-f",
            env_file.to_str().unwrap(),
            "--env-format",
        ])
        .assert()
        .success();

    // Check file content - should be NAME=value format
    let content = fs::read_to_string(&env_file).unwrap();
    assert!(content.starts_with("TEST_INJECT_KEY="));
    assert!(!content.contains("export"));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_INJECT_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_inject_env_format_with_export() {
    let _dir = setup_test_env();
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join("env.sh");

    // Create a test secret
    secret_agent()
        .args(["create", "TEST_EXPORT_KEY", "--force"])
        .assert()
        .success();

    // Inject with --env-format --export
    secret_agent()
        .args([
            "inject",
            "TEST_EXPORT_KEY",
            "-f",
            env_file.to_str().unwrap(),
            "--env-format",
            "--export",
        ])
        .assert()
        .success();

    // Check file content - should be export NAME=value format
    let content = fs::read_to_string(&env_file).unwrap();
    assert!(content.starts_with("export TEST_EXPORT_KEY="));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_EXPORT_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_exec_sanitizes_output() {
    let _dir = setup_test_env();

    // Create a test secret with known value
    secret_agent()
        .args(["create", "TEST_SANITIZE_KEY", "--force"])
        .assert()
        .success();

    // Run a command that would output the secret
    secret_agent()
        .args([
            "exec",
            "--env",
            "TEST_SANITIZE_KEY",
            "printenv",
            "TEST_SANITIZE_KEY",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:TEST_SANITIZE_KEY]"));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_SANITIZE_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_exec_with_multiple_env_flags() {
    let _dir = setup_test_env();

    // Create test secrets
    secret_agent()
        .args(["create", "TEST_MULTI_KEY1", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "TEST_MULTI_KEY2", "--force"])
        .assert()
        .success();

    // exec with multiple -e flags
    secret_agent()
        .args([
            "exec",
            "-e",
            "TEST_MULTI_KEY1",
            "-e",
            "TEST_MULTI_KEY2",
            "env",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("TEST_MULTI_KEY1="))
        .stdout(predicate::str::contains("TEST_MULTI_KEY2="));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_MULTI_KEY1"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "TEST_MULTI_KEY2"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_exec_with_env_rename() {
    let _dir = setup_test_env();

    // Create a test secret
    secret_agent()
        .args(["create", "TEST_RENAME_KEY", "--force"])
        .assert()
        .success();

    // exec with rename syntax VAULT_NAME:ENV_VAR
    // The env var MY_CUSTOM_VAR is set, and output is redacted using the secret name
    secret_agent()
        .args([
            "exec",
            "--env",
            "TEST_RENAME_KEY:MY_CUSTOM_VAR",
            "printenv",
            "MY_CUSTOM_VAR",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:"));

    // Cleanup
    secret_agent()
        .args(["delete", "TEST_RENAME_KEY"])
        .assert()
        .success();
}
