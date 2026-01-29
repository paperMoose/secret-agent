use crate::secret_gen::{self, Charset};
use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(name: &str, length: usize, charset: &str) -> Result<()> {
    let charset: Charset = charset
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))
        .context("invalid charset")?;

    let vault = Vault::open().context("failed to open vault")?;

    let value = secret_gen::generate(length, charset);
    vault
        .create(name, &value)
        .context("failed to create secret")?;

    println!("Created secret: {}", name);
    Ok(())
}
