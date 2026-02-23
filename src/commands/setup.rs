use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const MARKER: &str = "## Secrets Management (secret-agent)";

const CLAUDE_INSTRUCTIONS: &str = r#"## Secrets Management (secret-agent)

### Why use secret-agent
- Secrets never appear in agent context windows
- Traces are safe to log
- Prompt injection can't leak secrets you don't have
- Output is automatically sanitized (secrets replaced with `[REDACTED:NAME]`)

### Run commands with secrets (preferred)
```bash
# As environment variables (recommended)
secret-agent exec --env API_KEY node app.js
secret-agent exec -e KEY1 -e KEY2 ./script.sh

# With bucket prefix - env var is just the name (API_KEY, not prod/API_KEY)
secret-agent exec --env prod/API_KEY node app.js

# Rename: vault secret -> different env var name
secret-agent exec --env MY_SECRET:OPENAI_API_KEY python app.py

# Template secrets into command strings
secret-agent exec curl -H 'Authorization: Bearer {{API_KEY}}' https://api.example.com
```

### Create secrets
```bash
secret-agent create API_KEY                     # 32-char alphanumeric (default)
secret-agent create API_KEY --length 64         # Custom length
secret-agent create API_KEY --charset hex       # hex | base64 | ascii | alphanumeric
secret-agent create API_KEY --force             # Overwrite existing
```

### Import secrets
```bash
# From clipboard (clears after reading)
secret-agent import OPENAI_KEY --clipboard

# From stdin (single-line)
echo "sk-..." | secret-agent import API_KEY

# Replace existing secret
echo "new_value" | secret-agent import EXISTING_KEY --replace
```

### Import PEM files, certificates, and key pairs
```bash
# Pipe multiline content directly - full content is preserved
cat private_key.pem | secret-agent import TLS_KEY
cat certificate.pem | secret-agent import TLS_CERT

# Use --env to pass multiline secrets (NOT {{}} templates)
secret-agent exec --env TLS_KEY my-deploy-script
secret-agent exec --env TLS_CERT:SSL_CERT nginx -c /etc/nginx.conf
```
Note: `--env` is the correct way to use multiline secrets. The `{{PLACEHOLDER}}` template syntax is for single-line values like API keys only.

### List and delete
```bash
secret-agent list                    # All secrets
secret-agent list --bucket prod      # Only secrets in a bucket
secret-agent delete OLD_SECRET       # Remove permanently
```

### Buckets for organizing secrets
```bash
# Create secrets in buckets (bucket/name syntax)
secret-agent create prod/SUPABASE_KEY
secret-agent create dev/SUPABASE_KEY
secret-agent create staging/SUPABASE_KEY

# List by bucket
secret-agent list --bucket prod

# Bucket prefix is stripped for env vars
secret-agent exec --env prod/API_KEY node app.js  # env var = API_KEY
```

### Write secrets to files
```bash
# Append NAME=value to .env file
secret-agent inject DB_PASS --file .env --env-format

# Append export NAME="value" for shell scripts
secret-agent inject DB_PASS --file env.sh --env-format --export

# Replace a placeholder string in any file
secret-agent inject API_KEY --file config.json --placeholder __API_KEY__
```

### Bulk .env import/export
```bash
# Import all vars from a .env file into the vault
secret-agent env import --file .env.local

# Export specific secrets to .env
secret-agent env export --file .env API_KEY DB_PASS

# Export all secrets to .env
secret-agent env export --file .env --all
```

### Copy secret to clipboard (safe for agents)
```bash
# Agent can put a secret in the user's clipboard without ever seeing the value
secret-agent get API_KEY --clipboard

# Works on macOS (pbcopy) and Linux (X11/Wayland)
```

### Debug (not for agent use)
```bash
# Display a secret value (requires explicit safety flag)
secret-agent get API_KEY --unsafe-display
```

### Global flags
- `-q, --quiet` — Suppress informational output (for scripting)

### Avoiding Keychain Prompts

If macOS keychain prompts block automation, set:
```bash
export SECRET_AGENT_USE_FILE=1
```
This uses `~/.secret-agent/master.key` instead of the system keychain.
"#;

fn claude_md_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".claude").join("CLAUDE.md"))
}

/// Returns true if setup has already been completed.
pub fn is_configured() -> bool {
    let Some(path) = claude_md_path() else {
        return false;
    };
    match fs::read_to_string(path) {
        Ok(contents) => contents.contains(MARKER),
        Err(_) => false,
    }
}

pub fn run(print: bool, quiet: bool) -> Result<()> {
    if print {
        print!("{CLAUDE_INSTRUCTIONS}");
        return Ok(());
    }

    let path = claude_md_path().context("Could not determine home directory")?;

    // Create ~/.claude/ if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Check for existing instructions
    if path.exists() {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        if contents.contains(MARKER) {
            if !quiet {
                eprintln!("Already configured in {}", path.display());
            }
            return Ok(());
        }
    }

    // Append with a leading newline separator
    let mut content_to_append = String::from("\n");
    content_to_append.push_str(CLAUDE_INSTRUCTIONS);

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|_| {
            use std::io::Write;
            let mut f = fs::OpenOptions::new().append(true).open(&path)?;
            f.write_all(content_to_append.as_bytes())
        })
        .with_context(|| format!("Failed to write to {}", path.display()))?;

    if !quiet {
        eprintln!("Added secret-agent instructions to {}", path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper to override the path for testing
    fn run_with_path(print: bool, quiet: bool, path: &PathBuf) -> Result<()> {
        if print {
            print!("{CLAUDE_INSTRUCTIONS}");
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        if path.exists() {
            let contents = fs::read_to_string(path)?;
            if contents.contains(MARKER) {
                if !quiet {
                    eprintln!("Already configured in {}", path.display());
                }
                return Ok(());
            }
        }

        let mut content_to_append = String::from("\n");
        content_to_append.push_str(CLAUDE_INSTRUCTIONS);

        use std::io::Write;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        f.write_all(content_to_append.as_bytes())?;

        if !quiet {
            eprintln!("Added secret-agent instructions to {}", path.display());
        }

        Ok(())
    }

    #[test]
    fn test_creates_file_if_not_exists() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".claude").join("CLAUDE.md");

        run_with_path(false, true, &path).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains(MARKER));
        assert!(contents.contains("secret-agent exec --env API_KEY"));
    }

    #[test]
    fn test_appends_to_existing_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".claude");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("CLAUDE.md");

        fs::write(&path, "# Existing content\n\nSome stuff here.\n").unwrap();

        run_with_path(false, true, &path).unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.starts_with("# Existing content"));
        assert!(contents.contains(MARKER));
    }

    #[test]
    fn test_idempotent() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".claude");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("CLAUDE.md");

        run_with_path(false, true, &path).unwrap();
        let first = fs::read_to_string(&path).unwrap();

        run_with_path(false, true, &path).unwrap();
        let second = fs::read_to_string(&path).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn test_instructions_content() {
        assert!(CLAUDE_INSTRUCTIONS.contains(MARKER));
        assert!(CLAUDE_INSTRUCTIONS.contains("SECRET_AGENT_USE_FILE"));
        assert!(CLAUDE_INSTRUCTIONS.contains("secret-agent exec"));
        assert!(CLAUDE_INSTRUCTIONS.contains("secret-agent create"));
        assert!(CLAUDE_INSTRUCTIONS.contains("secret-agent import"));
    }
}
