use crate::error::Error;
use crate::sanitize;
use crate::vault::{secret_name_only, Vault};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

static PLACEHOLDER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\{\{(\w+)\}\}").expect("invalid placeholder regex"));

/// Parse an env spec like "SECRET_NAME", "bucket/SECRET_NAME", or "bucket/SECRET_NAME:ENV_VAR"
/// Returns (secret_path, env_var_name)
/// - "API_KEY" -> ("API_KEY", "API_KEY")
/// - "prod/API_KEY" -> ("prod/API_KEY", "API_KEY")
/// - "prod/API_KEY:MY_VAR" -> ("prod/API_KEY", "MY_VAR")
fn parse_env_spec(spec: &str) -> (String, String) {
    if let Some((secret, var)) = spec.split_once(':') {
        (secret.to_string(), var.to_string())
    } else {
        // Use just the secret name (without bucket) as the env var name
        let env_var = secret_name_only(spec).to_string();
        (spec.to_string(), env_var)
    }
}

/// Shell-quote an argument if it contains special characters
fn shell_quote(s: &str) -> String {
    // If the string is simple (alphanumeric, underscore, hyphen, dot, slash), use as-is
    if s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/') {
        return s.to_string();
    }
    // Otherwise, wrap in single quotes, escaping any existing single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn run(env_secrets: &[String], command_parts: &[String]) -> Result<i32> {
    let vault = Vault::open().context("failed to open vault")?;

    // Build the command string, properly quoting arguments that need it
    let command = command_parts
        .iter()
        .map(|s| shell_quote(s))
        .collect::<Vec<_>>()
        .join(" ");

    // Collect secrets needed for --env flags
    let mut env_vars: HashMap<String, String> = HashMap::new();
    let mut all_secrets: HashMap<String, String> = HashMap::new();

    for spec in env_secrets {
        let (secret_name, env_var_name) = parse_env_spec(spec);
        let value = vault.get(&secret_name).map_err(|e| match e {
            Error::SecretNotFound(_) => {
                anyhow::anyhow!("secret '{}' not found in vault", secret_name)
            }
            _ => anyhow::anyhow!("failed to get secret '{}': {}", secret_name, e),
        })?;
        env_vars.insert(env_var_name, value.clone());
        all_secrets.insert(secret_name, value);
    }

    // Parse placeholders from command (for backwards compatibility)
    let placeholder_names = parse_placeholders(&command);

    for name in &placeholder_names {
        if !all_secrets.contains_key(name) {
            let value = vault.get(name).map_err(|e| match e {
                Error::SecretNotFound(_) => {
                    anyhow::anyhow!("secret '{}' not found in vault", name)
                }
                _ => anyhow::anyhow!("failed to get secret '{}': {}", name, e),
            })?;
            all_secrets.insert(name.clone(), value);
        }
    }

    // Inject secrets into command string (for {{PLACEHOLDER}} syntax)
    let injected_command = inject_secrets(&command, &all_secrets);

    // Execute with env vars
    execute_command(&injected_command, &env_vars, &all_secrets)
}

fn parse_placeholders(command: &str) -> Vec<String> {
    let names: Vec<String> = PLACEHOLDER_RE
        .captures_iter(command)
        .map(|cap| cap[1].to_string())
        .collect();

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<String> = names
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect();

    unique
}

fn inject_secrets(command: &str, secrets: &HashMap<String, String>) -> String {
    let mut result = command.to_owned();

    for (name, value) in secrets {
        let placeholder = format!("{{{{{}}}}}", name);
        result = result.replace(&placeholder, value);
    }

    result
}

fn execute_command(
    command: &str,
    env_vars: &HashMap<String, String>,
    secrets: &HashMap<String, String>,
) -> Result<i32> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);

    // Inject environment variables
    for (var_name, value) in env_vars {
        cmd.env(var_name, value);
    }

    let output = cmd.output().context("failed to execute command")?;

    // Combine all secret values for sanitization
    let mut all_secret_values = secrets.clone();
    for (var_name, value) in env_vars {
        // Use var name as key for sanitization display
        all_secret_values.insert(var_name.clone(), value.clone());
    }

    // Sanitize and print stdout
    let stdout = sanitize::sanitize_bytes(&output.stdout, &all_secret_values);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    // Sanitize and print stderr
    let stderr = sanitize::sanitize_bytes(&output.stderr, &all_secret_values);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    // Return exit code
    Ok(output.status.code().unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_spec_simple() {
        let (secret, var) = parse_env_spec("API_KEY");
        assert_eq!(secret, "API_KEY");
        assert_eq!(var, "API_KEY");
    }

    #[test]
    fn test_parse_env_spec_renamed() {
        let (secret, var) = parse_env_spec("MY_SECRET:OPENAI_API_KEY");
        assert_eq!(secret, "MY_SECRET");
        assert_eq!(var, "OPENAI_API_KEY");
    }

    #[test]
    fn test_parse_env_spec_with_bucket() {
        let (secret, var) = parse_env_spec("prod/API_KEY");
        assert_eq!(secret, "prod/API_KEY");
        assert_eq!(var, "API_KEY"); // env var is just the name, not bucket/name
    }

    #[test]
    fn test_parse_env_spec_with_bucket_renamed() {
        let (secret, var) = parse_env_spec("prod/SECRET:MY_VAR");
        assert_eq!(secret, "prod/SECRET");
        assert_eq!(var, "MY_VAR");
    }

    #[test]
    fn test_parse_placeholders() {
        let cmd = "curl -H 'Auth: {{API_KEY}}' --data '{{DATA}}'";
        let names = parse_placeholders(cmd);
        assert_eq!(names, vec!["API_KEY", "DATA"]);
    }

    #[test]
    fn test_parse_placeholders_dedupe() {
        let cmd = "echo {{SECRET}} {{SECRET}} {{OTHER}}";
        let names = parse_placeholders(cmd);
        assert_eq!(names, vec!["SECRET", "OTHER"]);
    }

    #[test]
    fn test_parse_placeholders_empty() {
        let cmd = "echo hello world";
        let names = parse_placeholders(cmd);
        assert!(names.is_empty());
    }

    #[test]
    fn test_inject_secrets() {
        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "sk-12345".to_string());
        secrets.insert("HOST".to_string(), "example.com".to_string());

        let cmd = "curl https://{{HOST}}/api -H 'Auth: {{API_KEY}}'";
        let result = inject_secrets(cmd, &secrets);

        assert_eq!(result, "curl https://example.com/api -H 'Auth: sk-12345'");
    }

    #[test]
    fn test_shell_quote_simple() {
        assert_eq!(shell_quote("hello"), "hello");
        assert_eq!(shell_quote("file.txt"), "file.txt");
        assert_eq!(shell_quote("/path/to/file"), "/path/to/file");
        assert_eq!(shell_quote("my-name_123"), "my-name_123");
    }

    #[test]
    fn test_shell_quote_special_chars() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
        assert_eq!(shell_quote("echo \"test\""), "'echo \"test\"'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_quote_preserves_sh_c_args() {
        // When user runs: secret-agent exec sh -c 'echo "{{KEY}}"'
        // The shell passes: ["sh", "-c", "echo \"{{KEY}}\""]
        // We need to reconstruct: sh -c 'echo "{{KEY}}"'
        let parts = vec!["sh".to_string(), "-c".to_string(), "echo \"{{KEY}}\"".to_string()];
        let quoted: Vec<String> = parts.iter().map(|s| shell_quote(s)).collect();
        let command = quoted.join(" ");
        assert_eq!(command, "sh -c 'echo \"{{KEY}}\"'");
    }
}
