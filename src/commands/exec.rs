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
    // Empty string needs quoting
    if s.is_empty() {
        return "''".to_string();
    }
    // If the string is simple (alphanumeric, underscore, hyphen, dot, slash, colon), use as-is
    // Colon is safe in shell arguments (used in URLs, paths, etc.)
    if s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/' || c == ':') {
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

    #[test]
    fn test_shell_quote_empty_string() {
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn test_shell_quote_backticks() {
        assert_eq!(shell_quote("echo `whoami`"), "'echo `whoami`'");
    }

    #[test]
    fn test_shell_quote_dollar_signs() {
        assert_eq!(shell_quote("echo $HOME"), "'echo $HOME'");
        assert_eq!(shell_quote("${VAR}"), "'${VAR}'");
    }

    #[test]
    fn test_shell_quote_semicolons_and_pipes() {
        assert_eq!(shell_quote("cmd1; cmd2"), "'cmd1; cmd2'");
        assert_eq!(shell_quote("cmd1 | cmd2"), "'cmd1 | cmd2'");
        assert_eq!(shell_quote("cmd1 && cmd2"), "'cmd1 && cmd2'");
    }

    #[test]
    fn test_shell_quote_newlines() {
        assert_eq!(shell_quote("line1\nline2"), "'line1\nline2'");
    }

    #[test]
    fn test_shell_quote_mixed_quotes() {
        // Single quotes inside need escaping
        assert_eq!(shell_quote("it's a \"test\""), "'it'\\''s a \"test\"'");
    }

    #[test]
    fn test_shell_quote_parentheses() {
        assert_eq!(shell_quote("(subshell)"), "'(subshell)'");
        assert_eq!(shell_quote("$(command)"), "'$(command)'");
    }

    #[test]
    fn test_command_reconstruction_simple() {
        let parts = vec!["echo".to_string(), "hello".to_string()];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        assert_eq!(command, "echo hello");
    }

    #[test]
    fn test_command_reconstruction_with_flags() {
        let parts = vec!["curl".to_string(), "-X".to_string(), "POST".to_string(), "https://api.example.com".to_string()];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        assert_eq!(command, "curl -X POST https://api.example.com");
    }

    #[test]
    fn test_command_reconstruction_with_json() {
        // When user runs: secret-agent exec curl -d '{"key": "value"}'
        // Shell passes: ["curl", "-d", "{\"key\": \"value\"}"]
        let parts = vec!["curl".to_string(), "-d".to_string(), "{\"key\": \"value\"}".to_string()];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        assert_eq!(command, "curl -d '{\"key\": \"value\"}'");
    }

    #[test]
    fn test_command_reconstruction_piped_to_vercel() {
        // The exact use case that was broken:
        // secret-agent exec sh -c 'echo "{{KEY}}" | vercel env add KEY production'
        // Shell passes: ["sh", "-c", "echo \"{{KEY}}\" | vercel env add KEY production"]
        let parts = vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo \"{{KEY}}\" | vercel env add KEY production".to_string(),
        ];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        assert_eq!(command, "sh -c 'echo \"{{KEY}}\" | vercel env add KEY production'");
    }

    #[test]
    fn test_parse_placeholders_in_quoted_command() {
        // After shell_quote, placeholders should still be findable
        let parts = vec!["sh".to_string(), "-c".to_string(), "echo \"{{API_KEY}}\"".to_string()];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        let placeholders = parse_placeholders(&command);
        assert_eq!(placeholders, vec!["API_KEY"]);
    }

    #[test]
    fn test_inject_secrets_in_quoted_command() {
        let parts = vec!["sh".to_string(), "-c".to_string(), "echo \"{{API_KEY}}\"".to_string()];
        let command: String = parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");

        let mut secrets = HashMap::new();
        secrets.insert("API_KEY".to_string(), "sk-secret-123".to_string());

        let injected = inject_secrets(&command, &secrets);
        assert_eq!(injected, "sh -c 'echo \"sk-secret-123\"'");
    }

    #[test]
    fn test_full_pipeline_sh_c_echo() {
        // Simulate the full pipeline for: secret-agent exec sh -c 'echo "{{KEY}}"'
        let command_parts = vec!["sh".to_string(), "-c".to_string(), "echo \"{{KEY}}\"".to_string()];

        // Step 1: Build command with proper quoting
        let command: String = command_parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");
        assert_eq!(command, "sh -c 'echo \"{{KEY}}\"'");

        // Step 2: Parse placeholders
        let placeholders = parse_placeholders(&command);
        assert_eq!(placeholders, vec!["KEY"]);

        // Step 3: Inject secrets
        let mut secrets = HashMap::new();
        secrets.insert("KEY".to_string(), "my-secret-value".to_string());
        let injected = inject_secrets(&command, &secrets);
        assert_eq!(injected, "sh -c 'echo \"my-secret-value\"'");
    }

    #[test]
    fn test_full_pipeline_complex_command() {
        // Simulate: secret-agent exec sh -c 'curl -H "Auth: {{TOKEN}}" https://api.com | jq .data'
        let command_parts = vec![
            "sh".to_string(),
            "-c".to_string(),
            "curl -H \"Auth: {{TOKEN}}\" https://api.com | jq .data".to_string(),
        ];

        let command: String = command_parts.iter().map(|s| shell_quote(s)).collect::<Vec<_>>().join(" ");

        let mut secrets = HashMap::new();
        secrets.insert("TOKEN".to_string(), "bearer-xyz".to_string());

        let injected = inject_secrets(&command, &secrets);
        assert!(injected.contains("bearer-xyz"));
        assert!(injected.contains("jq .data"));
    }
}
