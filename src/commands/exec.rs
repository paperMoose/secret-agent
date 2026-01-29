use crate::error::Error;
use crate::sanitize;
use crate::vault::Vault;
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

pub fn run(command: &str) -> Result<i32> {
    let vault = Vault::open().context("failed to open vault")?;

    // Parse placeholders from command
    let placeholder_names = parse_placeholders(command)?;

    if placeholder_names.is_empty() {
        // No secrets needed, just run the command
        return execute_command(command, &HashMap::new());
    }

    // Fetch all required secrets
    let mut secrets = HashMap::new();
    for name in &placeholder_names {
        let value = vault
            .get(name)
            .map_err(|e| match e {
                Error::SecretNotFound(_) => {
                    anyhow::anyhow!("secret '{}' not found in vault", name)
                }
                _ => anyhow::anyhow!("failed to get secret '{}': {}", name, e),
            })?;
        secrets.insert(name.clone(), value);
    }

    // Inject secrets into command
    let injected_command = inject_secrets(command, &secrets);

    // Execute and return exit code
    execute_command(&injected_command, &secrets)
}

fn parse_placeholders(command: &str) -> Result<Vec<String>> {
    let re = Regex::new(r"\{\{(\w+)\}\}").context("failed to compile regex")?;

    let names: Vec<String> = re
        .captures_iter(command)
        .map(|cap| cap[1].to_string())
        .collect();

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<String> = names
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect();

    Ok(unique)
}

fn inject_secrets(command: &str, secrets: &HashMap<String, String>) -> String {
    let mut result = command.to_owned();

    for (name, value) in secrets {
        let placeholder = format!("{{{{{}}}}}", name);
        result = result.replace(&placeholder, value);
    }

    result
}

fn execute_command(command: &str, secrets: &HashMap<String, String>) -> Result<i32> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .context("failed to execute command")?;

    // Sanitize and print stdout
    let stdout = sanitize::sanitize_bytes(&output.stdout, secrets);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    // Sanitize and print stderr
    let stderr = sanitize::sanitize_bytes(&output.stderr, secrets);
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
    fn test_parse_placeholders() {
        let cmd = "curl -H 'Auth: {{API_KEY}}' --data '{{DATA}}'";
        let names = parse_placeholders(cmd).unwrap();
        assert_eq!(names, vec!["API_KEY", "DATA"]);
    }

    #[test]
    fn test_parse_placeholders_dedupe() {
        let cmd = "echo {{SECRET}} {{SECRET}} {{OTHER}}";
        let names = parse_placeholders(cmd).unwrap();
        assert_eq!(names, vec!["SECRET", "OTHER"]);
    }

    #[test]
    fn test_parse_placeholders_empty() {
        let cmd = "echo hello world";
        let names = parse_placeholders(cmd).unwrap();
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
}
