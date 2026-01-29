use crate::vault::Vault;
use anyhow::{Context, Result};
use std::io::{self, BufRead};

pub fn run(name: &str, clipboard: bool, quiet: bool) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    let value = if clipboard {
        read_from_clipboard()?
    } else {
        read_secret_value()?
    };

    if value.is_empty() {
        anyhow::bail!("secret value cannot be empty");
    }

    vault
        .create(name, &value)
        .context("failed to import secret")?;

    if !quiet {
        println!("Imported secret: {}", name);
    }
    Ok(())
}

fn read_from_clipboard() -> Result<String> {
    let mut clipboard = arboard::Clipboard::new().context("failed to access clipboard")?;

    let value = clipboard
        .get_text()
        .context("failed to read from clipboard (is it empty or non-text?)")?;

    // Clear clipboard after reading for security
    let _ = clipboard.clear();

    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        anyhow::bail!("clipboard is empty");
    }

    Ok(trimmed)
}

fn read_secret_value() -> Result<String> {
    // Check if stdin is a TTY (interactive) or piped
    if atty::is(atty::Stream::Stdin) {
        // Interactive prompt with hidden input
        let value = rpassword::prompt_password("Enter secret value: ")
            .context("failed to read secret value")?;
        Ok(value)
    } else {
        // Read from stdin (piped input)
        let stdin = io::stdin();
        let mut value = String::new();
        stdin
            .lock()
            .read_line(&mut value)
            .context("failed to read from stdin")?;

        // Trim trailing newline
        Ok(value
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string())
    }
}
