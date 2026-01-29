use crate::crypto;
use crate::error::{Error, Result};
use crate::keychain;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use secrecy::{ExposeSecret, SecretString};
use std::path::PathBuf;

const SCHEMA_VERSION: i64 = 1;

pub struct Secret {
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

pub struct Vault {
    conn: Connection,
    master_key: SecretString, // Zeroized on drop
}

impl Vault {
    /// Open the vault, creating it if it doesn't exist
    pub fn open() -> Result<Self> {
        let vault_path = get_vault_path()?;

        // Ensure parent directory exists
        if let Some(parent) = vault_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&vault_path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Initialize schema
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS secrets (
                name TEXT PRIMARY KEY,
                encrypted_value BLOB NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        // Check/set schema version
        init_schema_version(&conn)?;

        // Get or create master key
        let master_key = SecretString::from(keychain::get_or_create_master_key()?);

        Ok(Self { conn, master_key })
    }

    /// Create a new secret with the given value
    pub fn create(&self, name: &str, value: &str) -> Result<()> {
        self.create_internal(name, value, false)
    }

    /// Create a new secret, optionally overwriting existing
    pub fn create_or_update(&self, name: &str, value: &str) -> Result<()> {
        self.create_internal(name, value, true)
    }

    fn create_internal(&self, name: &str, value: &str, force: bool) -> Result<()> {
        validate_name(name)?;

        // Check if secret already exists
        if self.exists(name)? {
            if force {
                return self.update(name, value);
            }
            return Err(Error::SecretAlreadyExists(name.to_string()));
        }

        let encrypted = crypto::encrypt(value.as_bytes(), self.master_key.expose_secret())?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO secrets (name, encrypted_value, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, encrypted, now, now],
        )?;

        Ok(())
    }

    /// Get the decrypted value of a secret
    pub fn get(&self, name: &str) -> Result<String> {
        let encrypted: Vec<u8> = self
            .conn
            .query_row(
                "SELECT encrypted_value FROM secrets WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::SecretNotFound(name.to_string()),
                _ => Error::Database(e),
            })?;

        let decrypted = crypto::decrypt(&encrypted, self.master_key.expose_secret())?;
        String::from_utf8(decrypted).map_err(|e| Error::Decryption(e.to_string()))
    }

    /// List all secrets (metadata only, no values)
    pub fn list(&self) -> Result<Vec<Secret>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, created_at, updated_at FROM secrets ORDER BY name")?;

        let secrets = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let created_at: String = row.get(1)?;
                let updated_at: String = row.get(2)?;

                Ok(Secret {
                    name,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(secrets)
    }

    /// Delete a secret
    pub fn delete(&self, name: &str) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM secrets WHERE name = ?1", params![name])?;

        if rows == 0 {
            return Err(Error::SecretNotFound(name.to_string()));
        }

        Ok(())
    }

    /// Check if a secret exists
    pub fn exists(&self, name: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM secrets WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Update an existing secret's value
    pub fn update(&self, name: &str, value: &str) -> Result<()> {
        if !self.exists(name)? {
            return Err(Error::SecretNotFound(name.to_string()));
        }

        let encrypted = crypto::encrypt(value.as_bytes(), self.master_key.expose_secret())?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE secrets SET encrypted_value = ?1, updated_at = ?2 WHERE name = ?3",
            params![encrypted, now, name],
        )?;

        Ok(())
    }
}

fn init_schema_version(conn: &Connection) -> Result<()> {
    let version: Option<i64> = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    match version {
        None => {
            // First run - set schema version
            conn.execute(
                "INSERT INTO metadata (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }
        Some(v) if v < SCHEMA_VERSION => {
            // Future: run migrations here
            conn.execute(
                "UPDATE metadata SET value = ?1 WHERE key = 'schema_version'",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }
        _ => {}
    }

    Ok(())
}

fn get_vault_path() -> Result<PathBuf> {
    // Allow override via environment variable (useful for testing)
    if let Ok(path) = std::env::var("SECRET_AGENT_VAULT_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = dirs::home_dir().ok_or_else(|| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine home directory",
        ))
    })?;

    Ok(home.join(".secret-agent").join("vault.db"))
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidSecretName("name cannot be empty".to_string()));
    }

    // Only allow alphanumeric, underscores, and hyphens
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(Error::InvalidSecretName(
            "name can only contain alphanumeric characters, underscores, and hyphens".to_string(),
        ));
    }

    // Must start with a letter or underscore
    if let Some(first) = name.chars().next() {
        if !first.is_alphabetic() && first != '_' {
            return Err(Error::InvalidSecretName(
                "name must start with a letter or underscore".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_vault() -> (Vault, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path().join("vault.db");
        std::env::set_var("SECRET_AGENT_VAULT_PATH", vault_path.to_str().unwrap());
        std::env::set_var("SECRET_AGENT_PASSPHRASE", "test-passphrase");
        let vault = Vault::open().unwrap();
        (vault, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (vault, _temp) = setup_test_vault();

        vault.create("TEST_SECRET", "my-value").unwrap();
        let value = vault.get("TEST_SECRET").unwrap();

        assert_eq!(value, "my-value");
    }

    #[test]
    fn test_create_duplicate_fails() {
        let (vault, _temp) = setup_test_vault();

        vault.create("TEST_SECRET", "value1").unwrap();
        let result = vault.create("TEST_SECRET", "value2");

        assert!(matches!(result, Err(Error::SecretAlreadyExists(_))));
    }

    #[test]
    fn test_get_nonexistent_fails() {
        let (vault, _temp) = setup_test_vault();

        let result = vault.get("NONEXISTENT");

        assert!(matches!(result, Err(Error::SecretNotFound(_))));
    }

    #[test]
    fn test_list() {
        let (vault, _temp) = setup_test_vault();

        vault.create("SECRET_A", "value-a").unwrap();
        vault.create("SECRET_B", "value-b").unwrap();

        let secrets = vault.list().unwrap();

        assert_eq!(secrets.len(), 2);
        assert_eq!(secrets[0].name, "SECRET_A");
        assert_eq!(secrets[1].name, "SECRET_B");
    }

    #[test]
    fn test_delete() {
        let (vault, _temp) = setup_test_vault();

        vault.create("TO_DELETE", "value").unwrap();
        assert!(vault.exists("TO_DELETE").unwrap());

        vault.delete("TO_DELETE").unwrap();
        assert!(!vault.exists("TO_DELETE").unwrap());
    }

    #[test]
    fn test_validate_name() {
        assert!(validate_name("VALID_NAME").is_ok());
        assert!(validate_name("valid-name").is_ok());
        assert!(validate_name("_private").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("123invalid").is_err());
        assert!(validate_name("has spaces").is_err());
    }
}
