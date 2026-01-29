use crate::error::{Error, Result};
use crate::secret_gen;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const SERVICE_NAME: &str = "secret-agent";
const MASTER_KEY_NAME: &str = "master-key";
const MASTER_KEY_LENGTH: usize = 32;

/// Get the master key with fallback chain:
/// 1. Environment variable SECRET_AGENT_PASSPHRASE (for CI/scripts)
/// 2. File-based key if SECRET_AGENT_USE_FILE=1 (skip keychain prompts)
/// 3. System keychain (macOS Keychain, Linux Secret Service)
/// 4. File at ~/.secret-agent/master.key (headless fallback)
/// 5. Interactive passphrase prompt (last resort)
pub fn get_or_create_master_key() -> Result<String> {
    // 1. Check environment variable first (highest priority for CI/automation)
    if let Ok(key) = std::env::var("SECRET_AGENT_PASSPHRASE") {
        return Ok(key);
    }

    // 2. If user prefers file-based storage (avoids keychain prompts)
    if std::env::var("SECRET_AGENT_USE_FILE").is_ok() {
        return get_or_create_file_key();
    }

    // 3. Try system keychain
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

    // Write key to file with restrictive permissions set atomically
    #[cfg(unix)]
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // Set permissions atomically on creation
            .open(&path)?;
        file.write_all(key.as_bytes())?;
    }

    #[cfg(not(unix))]
    {
        fs::write(&path, key)?;
    }

    eprintln!("Created master key file at {} (chmod 600)", path.display());

    Ok(())
}

fn should_use_file_fallback() -> bool {
    // Use file fallback on headless systems (no TTY and no keychain)
    !atty::is(atty::Stream::Stdin) || std::env::var("SSH_TTY").is_ok()
}

fn get_or_create_file_key() -> Result<String> {
    if let Some(key) = get_from_file()? {
        return Ok(key);
    }

    // Generate and store new key
    let key = secret_gen::generate(MASTER_KEY_LENGTH, secret_gen::Charset::Alphanumeric);
    store_in_file(&key)?;
    Ok(key)
}

fn prompt_for_passphrase() -> Result<String> {
    eprintln!("No keychain available. Please enter a passphrase for the vault:");
    eprintln!("(Tip: Set SECRET_AGENT_PASSPHRASE env var to skip this prompt)");

    let passphrase = rpassword::prompt_password("Passphrase: ").map_err(|e| Error::Io(e))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn store_in_file_at(path: &std::path::Path, key: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        #[cfg(unix)]
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)?;
            file.write_all(key.as_bytes())?;
        }

        #[cfg(not(unix))]
        {
            fs::write(path, key)?;
        }

        Ok(())
    }

    fn get_from_file_at(path: &std::path::Path) -> Result<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }

        #[cfg(unix)]
        {
            let metadata = fs::metadata(path)?;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(Error::Keychain(format!(
                    "master key file has insecure permissions {:o}, expected 600",
                    mode & 0o777
                )));
            }
        }

        let content = fs::read_to_string(path)?;
        Ok(Some(content.trim().to_string()))
    }

    #[test]
    fn test_file_storage_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("master.key");

        let original_key = "test-master-key-12345";
        store_in_file_at(&key_path, original_key).unwrap();

        let retrieved = get_from_file_at(&key_path).unwrap();
        assert_eq!(retrieved, Some(original_key.to_string()));
    }

    #[test]
    fn test_file_not_found_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("nonexistent.key");

        let result = get_from_file_at(&key_path).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[cfg(unix)]
    fn test_rejects_insecure_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("insecure.key");

        // Write file with insecure permissions (644)
        fs::write(&key_path, "secret-key").unwrap();
        let mut perms = fs::metadata(&key_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&key_path, perms).unwrap();

        let result = get_from_file_at(&key_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("insecure permissions"));
    }

    #[test]
    #[cfg(unix)]
    fn test_file_created_with_600_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("secure.key");

        store_in_file_at(&key_path, "test-key").unwrap();

        let metadata = fs::metadata(&key_path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "Expected 600 permissions, got {:o}", mode);
    }

    #[test]
    fn test_file_storage_trims_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join("master.key");

        // Write key with trailing newline
        #[cfg(unix)]
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .mode(0o600)
                .open(&key_path)
                .unwrap();
            file.write_all(b"my-key-value\n").unwrap();
        }

        #[cfg(not(unix))]
        {
            fs::write(&key_path, "my-key-value\n").unwrap();
        }

        let retrieved = get_from_file_at(&key_path).unwrap();
        assert_eq!(retrieved, Some("my-key-value".to_string()));
    }
}
