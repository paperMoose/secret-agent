use crate::vault::Vault;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn export(file: &str, names: &[String], all: bool, quiet: bool) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    let secrets_to_export: Vec<String> = if all {
        vault.list()?.into_iter().map(|s| s.name).collect()
    } else {
        names.to_vec()
    };

    if secrets_to_export.is_empty() {
        if !quiet {
            println!("No secrets to export.");
        }
        return Ok(());
    }

    let path = Path::new(file);
    let mut lines: Vec<String> = Vec::new();

    for name in &secrets_to_export {
        let value = vault
            .get(name)
            .with_context(|| format!("failed to get secret '{}'", name))?;
        lines.push(format!("{}={}", name, quote_env_value(&value)));
    }

    let content = lines.join("\n") + "\n";
    fs::write(path, content)
        .with_context(|| format!("failed to write file: {}", path.display()))?;

    if !quiet {
        println!("Exported {} secrets to {}", secrets_to_export.len(), file);
    }
    Ok(())
}

pub fn import(file: &str, quiet: bool) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    let content =
        fs::read_to_string(file).with_context(|| format!("failed to read file: {}", file))?;

    let mut imported = Vec::new();
    let mut skipped = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse NAME=value
        if let Some((name, value)) = parse_env_line(line) {
            // Check if secret already exists
            if vault.exists(&name)? {
                skipped.push(name);
                continue;
            }

            vault
                .create(&name, &value)
                .with_context(|| format!("failed to import '{}'", name))?;
            imported.push(name);
        }
    }

    if !quiet {
        if imported.is_empty() && skipped.is_empty() {
            println!("No secrets found in {}", file);
        } else {
            if !imported.is_empty() {
                println!(
                    "Imported {} secrets: {}",
                    imported.len(),
                    imported.join(", ")
                );
            }
            if !skipped.is_empty() {
                println!(
                    "Skipped {} existing secrets: {}",
                    skipped.len(),
                    skipped.join(", ")
                );
            }
        }
    }

    Ok(())
}

fn parse_env_line(line: &str) -> Option<(String, String)> {
    // Handle "export NAME=value" format
    let line = line.strip_prefix("export ").unwrap_or(line);

    let (name, value) = line.split_once('=')?;
    let name = name.trim().to_string();
    let value = unquote_env_value(value.trim());

    // Validate name
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }

    Some((name, value))
}

fn unquote_env_value(value: &str) -> String {
    let value = value.trim();

    // Handle quoted strings
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        let inner = &value[1..value.len() - 1];
        // Unescape common sequences
        return inner
            .replace("\\n", "\n")
            .replace("\\\"", "\"")
            .replace("\\'", "'")
            .replace("\\$", "$")
            .replace("\\\\", "\\");
    }

    value.to_string()
}

fn quote_env_value(value: &str) -> String {
    // If value contains spaces, quotes, or special chars, wrap in quotes
    if value.contains(' ')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('$')
        || value.contains('\n')
        || value.contains('#')
    {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
            .replace('\n', "\\n");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_line() {
        assert_eq!(
            parse_env_line("API_KEY=sk-12345"),
            Some(("API_KEY".to_string(), "sk-12345".to_string()))
        );

        assert_eq!(
            parse_env_line("export DB_PASS=hunter2"),
            Some(("DB_PASS".to_string(), "hunter2".to_string()))
        );

        assert_eq!(
            parse_env_line("QUOTED=\"hello world\""),
            Some(("QUOTED".to_string(), "hello world".to_string()))
        );

        assert_eq!(parse_env_line("# comment"), None);
        assert_eq!(parse_env_line(""), None);
        assert_eq!(parse_env_line("invalid line"), None);
    }

    #[test]
    fn test_unquote_env_value() {
        assert_eq!(unquote_env_value("simple"), "simple");
        assert_eq!(unquote_env_value("\"quoted\""), "quoted");
        assert_eq!(unquote_env_value("'single'"), "single");
        assert_eq!(unquote_env_value("\"with\\nnewline\""), "with\nnewline");
        assert_eq!(unquote_env_value("\"with\\\"quote\""), "with\"quote");
    }

    #[test]
    fn test_quote_env_value() {
        assert_eq!(quote_env_value("simple"), "simple");
        assert_eq!(quote_env_value("has space"), "\"has space\"");
        assert_eq!(quote_env_value("has$var"), "\"has\\$var\"");
    }
}
