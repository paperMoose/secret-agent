use crate::error::{Error, Result};
use crate::secret_gen;

const SERVICE_NAME: &str = "secret-agent";
const MASTER_KEY_NAME: &str = "master-key";
const MASTER_KEY_LENGTH: usize = 32;

/// Get the master key from the system keychain, creating it if it doesn't exist.
/// Falls back to prompting for a passphrase if keychain is unavailable.
pub fn get_or_create_master_key() -> Result<String> {
    match get_from_keychain() {
        Ok(Some(key)) => Ok(key),
        Ok(None) => {
            // First run - generate and store master key
            let key = secret_gen::generate(MASTER_KEY_LENGTH, secret_gen::Charset::Alphanumeric);
            store_in_keychain(&key)?;
            Ok(key)
        }
        Err(_) => {
            // Keychain unavailable - check for environment variable or prompt
            if let Ok(key) = std::env::var("SECRET_AGENT_PASSPHRASE") {
                return Ok(key);
            }
            prompt_for_passphrase()
        }
    }
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

fn prompt_for_passphrase() -> Result<String> {
    eprintln!("Keychain unavailable. Please enter a passphrase for the vault:");
    let passphrase = rpassword::prompt_password("Passphrase: ")
        .map_err(|e| Error::Io(e))?;

    if passphrase.is_empty() {
        return Err(Error::Keychain("passphrase cannot be empty".to_string()));
    }

    Ok(passphrase)
}

/// Delete the master key from the keychain (for testing/reset)
#[allow(dead_code)]
pub fn delete_master_key() -> Result<()> {
    let entry = keyring::Entry::new(SERVICE_NAME, MASTER_KEY_NAME)
        .map_err(|e| Error::Keychain(e.to_string()))?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
        Err(e) => Err(Error::Keychain(e.to_string())),
    }
}
