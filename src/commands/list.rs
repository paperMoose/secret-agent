use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    let secrets = vault.list().context("failed to list secrets")?;

    if secrets.is_empty() {
        println!("No secrets stored.");
        return Ok(());
    }

    // Print header
    println!("{:<24} {}", "NAME", "CREATED");

    for secret in secrets {
        let created = secret.created_at.format("%Y-%m-%d %H:%M:%S");
        println!("{:<24} {}", secret.name, created);
    }

    Ok(())
}
