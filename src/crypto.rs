use crate::error::{Error, Result};
use age::secrecy::SecretString;
use std::io::{Read, Write};

/// Encrypt plaintext using age with a passphrase (scrypt-based)
pub fn encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor = age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_owned()));

    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|e| Error::Encryption(e.to_string()))?;

    writer
        .write_all(plaintext)
        .map_err(|e| Error::Encryption(e.to_string()))?;

    writer
        .finish()
        .map_err(|e| Error::Encryption(e.to_string()))?;

    Ok(encrypted)
}

/// Decrypt ciphertext using age with a passphrase (scrypt-based)
pub fn decrypt(ciphertext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let decryptor =
        age::Decryptor::new(ciphertext).map_err(|e| Error::Decryption(e.to_string()))?;

    let mut decrypted = vec![];
    let mut reader = decryptor
        .decrypt(std::iter::once(
            &age::scrypt::Identity::new(SecretString::from(passphrase.to_owned()))
                as &dyn age::Identity,
        ))
        .map_err(|e| Error::Decryption(e.to_string()))?;

    reader
        .read_to_end(&mut decrypted)
        .map_err(|e| Error::Decryption(e.to_string()))?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"my-secret-value";
        let passphrase = "test-passphrase";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_passphrase_fails() {
        let plaintext = b"my-secret-value";
        let passphrase = "correct-passphrase";
        let wrong_passphrase = "wrong-passphrase";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let result = decrypt(&encrypted, wrong_passphrase);

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let plaintext = b"";
        let passphrase = "test-passphrase";

        let encrypted = encrypt(plaintext, passphrase).unwrap();
        let decrypted = decrypt(&encrypted, passphrase).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
