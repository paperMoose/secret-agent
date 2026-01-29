use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(name: &str, quiet: bool) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    vault.delete(name).context("failed to delete secret")?;

    if !quiet {
        println!("Deleted secret: {}", name);
    }
    Ok(())
}
