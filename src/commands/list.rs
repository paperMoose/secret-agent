use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(bucket: Option<&str>) -> Result<()> {
    let vault = Vault::open().context("failed to open vault")?;

    let secrets = vault
        .list_by_bucket(bucket)
        .context("failed to list secrets")?;

    if secrets.is_empty() {
        if let Some(b) = bucket {
            println!("No secrets in bucket '{}'.", b);
        } else {
            println!("No secrets stored.");
        }
        return Ok(());
    }

    // Print header
    println!("{:<32} CREATED", "NAME");

    for secret in secrets {
        let created = secret.created_at.format("%Y-%m-%d %H:%M:%S");
        println!("{:<32} {}", secret.name, created);
    }

    Ok(())
}
