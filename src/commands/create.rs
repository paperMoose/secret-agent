use crate::secret_gen::{self, Charset};
use crate::vault::Vault;
use anyhow::{Context, Result};

pub fn run(name: &str, length: usize, charset: &str, force: bool, quiet: bool) -> Result<()> {
    let charset: Charset = charset
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))
        .context("invalid charset")?;

    let vault = Vault::open().context("failed to open vault")?;

    let value = secret_gen::generate(length, charset);

    if force {
        vault
            .create_or_update(name, &value)
            .context("failed to create secret")?;
    } else {
        vault
            .create(name, &value)
            .context("failed to create secret")?;
    }

    if !quiet {
        println!("Created secret: {}", name);
    }
    Ok(())
}
