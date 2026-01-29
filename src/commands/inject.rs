use crate::vault::Vault;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn run(
    name: &str,
    file: &str,
    placeholder: Option<&str>,
    env_format: bool,
    quiet: bool,
) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;
    let value = vault.get(name).context("failed to get secret")?;

    let path = Path::new(file);

    if env_format {
        // Append or update NAME=value line
        inject_env_format(path, name, &value)?;
    } else if let Some(placeholder) = placeholder {
        // Replace placeholder in file
        inject_placeholder(path, placeholder, &value)?;
    } else {
        anyhow::bail!("either --placeholder or --env-format is required");
    }

    if !quiet {
        println!("Injected {} into {}", name, file);
    }
    Ok(())
}

fn inject_placeholder(path: &Path, placeholder: &str, value: &str) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read file: {}", path.display()))?;

    if !content.contains(placeholder) {
        anyhow::bail!(
            "placeholder '{}' not found in file: {}",
            placeholder,
            path.display()
        );
    }

    let new_content = content.replace(placeholder, value);

    fs::write(path, new_content)
        .with_context(|| format!("failed to write file: {}", path.display()))?;

    Ok(())
}

fn inject_env_format(path: &Path, name: &str, value: &str) -> Result<()> {
    let mut content = if path.exists() {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read file: {}", path.display()))?
    } else {
        String::new()
    };

    // Check if the variable already exists
    let var_pattern = format!("{}=", name);
    let mut found = false;
    let mut new_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.starts_with(&var_pattern) || line.starts_with(&format!("export {}=", name)) {
            // Replace existing line
            new_lines.push(format!("{}={}", name, quote_env_value(value)));
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        // Append new line
        new_lines.push(format!("{}={}", name, quote_env_value(value)));
    }

    content = new_lines.join("\n");

    // Ensure file ends with newline
    if !content.ends_with('\n') {
        content.push('\n');
    }

    fs::write(path, content)
        .with_context(|| format!("failed to write file: {}", path.display()))?;

    Ok(())
}

/// Quote value for .env file if needed
fn quote_env_value(value: &str) -> String {
    // If value contains spaces, quotes, or special chars, wrap in quotes
    if value.contains(' ')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('$')
        || value.contains('\n')
        || value.contains('#')
    {
        // Use double quotes and escape special characters
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_inject_placeholder() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "password={{{{DB_PASS}}}}").unwrap();

        inject_placeholder(file.path(), "{{DB_PASS}}", "secret123").unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert_eq!(content.trim(), "password=secret123");
    }

    #[test]
    fn test_inject_env_format_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");

        inject_env_format(&path, "API_KEY", "sk-12345").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "API_KEY=sk-12345\n");
    }

    #[test]
    fn test_inject_env_format_existing_var() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "API_KEY=old-value").unwrap();
        writeln!(file, "OTHER=keep").unwrap();

        inject_env_format(file.path(), "API_KEY", "new-value").unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("API_KEY=new-value"));
        assert!(content.contains("OTHER=keep"));
        assert!(!content.contains("old-value"));
    }

    #[test]
    fn test_inject_env_format_append() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "EXISTING=value").unwrap();

        inject_env_format(file.path(), "NEW_KEY", "new-value").unwrap();

        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("EXISTING=value"));
        assert!(content.contains("NEW_KEY=new-value"));
    }

    #[test]
    fn test_quote_env_value() {
        assert_eq!(quote_env_value("simple"), "simple");
        assert_eq!(quote_env_value("has space"), "\"has space\"");
        assert_eq!(quote_env_value("has$dollar"), "\"has\\$dollar\"");
        assert_eq!(quote_env_value("has\"quote"), "\"has\\\"quote\"");
    }
}
