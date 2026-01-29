use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(name: &str, unsafe_display: bool) -> Result<()> {
    if !unsafe_display {
        anyhow::bail!(
            "You must use --unsafe-display to show secret values.\n\
             WARNING: This will display the secret in plaintext.\n\
             Do not use in agent contexts or logged sessions."
        );
    }

    let vault = Vault::open().context("failed to open vault")?;
    let value = vault.get(name).context("failed to get secret")?;

    eprintln!("WARNING: Displaying secret value. Do not use in agent contexts.");
    println!("{}", value);

    Ok(())
}
