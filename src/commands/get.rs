use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(name: &str, clipboard: bool, unsafe_display: bool, quiet: bool) -> Result<()> {
    if !clipboard && !unsafe_display {
        anyhow::bail!(
            "You must use --clipboard or --unsafe-display to retrieve a secret.\n\
             --clipboard copies to clipboard (safe for agents)\n\
             --unsafe-display prints to stdout (NOT for agent use)"
        );
    }

    let vault = Vault::open().context("failed to open vault")?;
    let value = vault.get(name).context("failed to get secret")?;

    if clipboard {
        let mut cb = arboard::Clipboard::new().context("failed to access clipboard")?;
        cb.set_text(&value)
            .context("failed to copy secret to clipboard")?;
        if !quiet {
            println!("Copied {} to clipboard", name);
        }
    } else {
        eprintln!("WARNING: Displaying secret value. Do not use in agent contexts.");
        println!("{}", value);
    }

    Ok(())
}
