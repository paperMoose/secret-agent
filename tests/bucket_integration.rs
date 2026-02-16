use assert_cmd::Command;
use predicates::prelude::*;
use serial_test::serial;
use tempfile::TempDir;

fn secret_agent() -> Command {
    Command::cargo_bin("secret-agent").unwrap()
}

fn setup_test_env() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::env::set_var("SECRET_AGENT_VAULT_PATH", dir.path().join("vault.db"));
    std::env::set_var("SECRET_AGENT_USE_FILE", "1");
    dir
}

#[test]
#[serial]
fn test_create_secret_with_bucket() {
    let _dir = setup_test_env();

    // Create secrets in different buckets
    secret_agent()
        .args(["create", "prod/API_KEY", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created secret: prod/API_KEY"));

    secret_agent()
        .args(["create", "dev/API_KEY", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created secret: dev/API_KEY"));

    // Both should be listed
    secret_agent()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod/API_KEY"))
        .stdout(predicate::str::contains("dev/API_KEY"));

    // Cleanup
    secret_agent()
        .args(["delete", "prod/API_KEY"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "dev/API_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_list_with_bucket_filter() {
    let _dir = setup_test_env();

    // Create secrets in different buckets
    secret_agent()
        .args(["create", "prod/KEY1", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "prod/KEY2", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "dev/KEY1", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "GLOBAL_KEY", "--force"])
        .assert()
        .success();

    // List only prod bucket
    secret_agent()
        .args(["list", "--bucket", "prod"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod/KEY1"))
        .stdout(predicate::str::contains("prod/KEY2"))
        .stdout(predicate::str::contains("dev/KEY1").not())
        .stdout(predicate::str::contains("GLOBAL_KEY").not());

    // List only dev bucket
    secret_agent()
        .args(["list", "--bucket", "dev"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dev/KEY1"))
        .stdout(predicate::str::contains("prod/KEY1").not());

    // Cleanup
    secret_agent()
        .args(["delete", "prod/KEY1"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "prod/KEY2"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "dev/KEY1"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "GLOBAL_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_exec_with_bucket_secret() {
    let _dir = setup_test_env();

    // Create secrets in different buckets
    secret_agent()
        .args(["create", "prod/DB_PASS", "--force"])
        .assert()
        .success();

    // exec should work with bucket syntax
    secret_agent()
        .args(["exec", "--env", "prod/DB_PASS", "printenv", "DB_PASS"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:"));

    // Cleanup
    secret_agent()
        .args(["delete", "prod/DB_PASS"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_exec_with_bucket_secret_rename() {
    let _dir = setup_test_env();

    // Create secret in bucket
    secret_agent()
        .args(["create", "prod/SECRET", "--force"])
        .assert()
        .success();

    // exec with rename: bucket/secret:ENV_VAR
    secret_agent()
        .args(["exec", "--env", "prod/SECRET:MY_VAR", "printenv", "MY_VAR"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:"));

    // Cleanup
    secret_agent()
        .args(["delete", "prod/SECRET"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_import_with_bucket() {
    let _dir = setup_test_env();

    // Import secret into bucket
    secret_agent()
        .args(["import", "staging/TOKEN"])
        .write_stdin("my_token_value\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported secret: staging/TOKEN"));

    // Should be listed
    secret_agent()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("staging/TOKEN"));

    // Cleanup
    secret_agent()
        .args(["delete", "staging/TOKEN"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_inject_with_bucket() {
    let _dir = setup_test_env();
    let temp_dir = TempDir::new().unwrap();
    let env_file = temp_dir.path().join(".env");

    // Create secret in bucket
    secret_agent()
        .args(["create", "prod/API_KEY", "--force"])
        .assert()
        .success();

    // Inject should work - uses secret name without bucket for env var name
    secret_agent()
        .args([
            "inject",
            "prod/API_KEY",
            "-f",
            env_file.to_str().unwrap(),
            "--env-format",
        ])
        .assert()
        .success();

    // Check file content - should use API_KEY not prod/API_KEY
    let content = std::fs::read_to_string(&env_file).unwrap();
    assert!(content.contains("API_KEY="));
    assert!(!content.contains("prod/API_KEY="));

    // Cleanup
    secret_agent()
        .args(["delete", "prod/API_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_same_name_different_buckets() {
    let _dir = setup_test_env();

    // Create same-named secrets in different buckets
    secret_agent()
        .args(["create", "prod/SUPABASE_KEY", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "dev/SUPABASE_KEY", "--force"])
        .assert()
        .success();
    secret_agent()
        .args(["create", "staging/SUPABASE_KEY", "--force"])
        .assert()
        .success();

    // All three should exist
    secret_agent()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod/SUPABASE_KEY"))
        .stdout(predicate::str::contains("dev/SUPABASE_KEY"))
        .stdout(predicate::str::contains("staging/SUPABASE_KEY"));

    // Deleting one shouldn't affect others
    secret_agent()
        .args(["delete", "dev/SUPABASE_KEY"])
        .assert()
        .success();

    secret_agent()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prod/SUPABASE_KEY"))
        .stdout(predicate::str::contains("dev/SUPABASE_KEY").not())
        .stdout(predicate::str::contains("staging/SUPABASE_KEY"));

    // Cleanup
    secret_agent()
        .args(["delete", "prod/SUPABASE_KEY"])
        .assert()
        .success();
    secret_agent()
        .args(["delete", "staging/SUPABASE_KEY"])
        .assert()
        .success();
}

#[test]
#[serial]
fn test_backwards_compatibility_no_bucket() {
    let _dir = setup_test_env();

    // Secrets without bucket should still work
    secret_agent()
        .args(["create", "SIMPLE_KEY", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created secret: SIMPLE_KEY"));

    // Should be listed without bucket prefix
    secret_agent()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SIMPLE_KEY"));

    // exec should work
    secret_agent()
        .args(["exec", "--env", "SIMPLE_KEY", "printenv", "SIMPLE_KEY"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[REDACTED:"));

    // Cleanup
    secret_agent()
        .args(["delete", "SIMPLE_KEY"])
        .assert()
        .success();
}
