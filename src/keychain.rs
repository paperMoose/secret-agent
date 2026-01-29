use crate::error::{Error, Result};
use crate::secret_gen;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const SERVICE_NAME: &str = "secret-agent";
const MASTER_KEY_NAME: &str = "master-key";
const MASTER_KEY_LENGTH: usize = 32;

/// Get the master key with fallback chain:
/// 1. Environment variable SECRET_AGENT_PASSPHRASE (for CI/scripts)
/// 2. System keychain (macOS Keychain, Linux Secret Service)
/// 3. Encrypted file at ~/.secret-agent/master.key (headless Linux)
/// 4. Interactive passphrase prompt (last resort)
pub fn get_or_create_master_key() -> Result<String> {
    // 1. Check environment variable first (highest priority for CI/automation)
    if let Ok(key) = std::env::var("SECRET_AGENT_PASSPHRASE") {
        return Ok(key);
    }

    // 2. Try system keychain
    match get_from_keychain() {
        Ok(Some(key)) => return Ok(key),
        Ok(None) => {
            // First run - generate and try to store in keychain
            let key = secret_gen::generate(MASTER_KEY_LENGTH, secret_gen::Charset::Alphanumeric);
            if store_in_keychain(&key).is_ok() {
                return Ok(key);
            }
            // Keychain store failed, try file fallback
            store_in_file(&key)?;
            return Ok(key);
        }
        Err(_) => {
            // Keychain unavailable, try file fallback
        }
    }

    // 3. Try file-based key (for headless Linux)
    if let Ok(Some(key)) = get_from_file() {
        return Ok(key);
    }

    // Check if we should create a new file-based key
    if should_use_file_fallback() {
        let key = secret_gen::generate(MASTER_KEY_LENGTH, secret_gen::Charset::Alphanumeric);
        store_in_file(&key)?;
        return Ok(key);
    }

    // 4. Last resort: prompt for passphrase
    prompt_for_passphrase()
}

fn get_from_keychain() -> Result<Option<String>> {
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_NAME)
        .map_err(|e| Error::Keychain(e.to_string()))?;

    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Error::Keychain(e.to_string())),
    }
}

fn store_in_keychain(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_NAME)
        .map_err(|e| Error::Keychain(e.to_string()))?;

    entry
        .set_password(key)
        .map_err(|e| Error::Keychain(e.to_string()))
}

fn get_key_file_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine home directory",
        ))
    })?;
    Ok(home.join(".secret-agent").join("master.key"))
}

fn get_from_file() -> Result<Option<String>> {
    let path = get_key_file_path()?;

    if !path.exists() {
        return Ok(None);
    }

    // Verify file permissions (should be 600)
    #[cfg(unix)]
    {
        let metadata = fs::metadata(&path)?;
        let mode = metadata.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(Error::Keychain(format!(
                "master key file has insecure permissions {:o}, expected 600",
                mode & 0o777
            )));
        }
    }

    let content = fs::read_to_string(&path)?;
    Ok(Some(content.trim().to_string()))
}

fn store_in_file(key: &str) -> Result<()> {
    let path = get_key_file_path()?;

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write key to file
    fs::write(&path, key)?;

    // Set restrictive permissions (600 = owner read/write only)
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    eprintln!(
        "Created master key file at {} (chmod 600)",
        path.display()
    );

    Ok(())
}

fn should_use_file_fallback() -> bool {
    // Use file fallback on headless systems (no TTY and no keychain)
    !atty::is(atty::Stream::Stdin) || std::env::var("SSH_TTY").is_ok()
}

fn prompt_for_passphrase() -> Result<String> {
    eprintln!("No keychain available. Please enter a passphrase for the vault:");
    eprintln!("(Tip: Set SECRET_AGENT_PASSPHRASE env var to skip this prompt)");

    let passphrase = rpassword::prompt_password("Passphrase: ")
        .map_err(|e| Error::Io(e))?;

    if passphrase.is_empty() {
        return Err(Error::Keychain("passphrase cannot be empty".to_string()));
    }

    Ok(passphrase)
}

/// Delete the master key from all storage locations
#[allow(dead_code)]
pub fn delete_master_key() -> Result<()> {
    // Try keychain
    let _ = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_NAME)
        .and_then(|entry| entry.delete_credential());

    // Try file
    if let Ok(path) = get_key_file_path() {
        let _ = fs::remove_file(path);
    }

    Ok(())
}
