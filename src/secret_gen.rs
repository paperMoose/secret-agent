use rand::Rng;

#[derive(Debug, Clone, Copy, Default)]
pub enum Charset {
    #[default]
    Alphanumeric,
    Ascii,
    Hex,
    Base64,
}

impl std::str::FromStr for Charset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "alphanumeric" => Ok(Charset::Alphanumeric),
            "ascii" => Ok(Charset::Ascii),
            "hex" => Ok(Charset::Hex),
            "base64" => Ok(Charset::Base64),
            _ => Err(format!("unknown charset: {}", s)),
        }
    }
}

const ALPHANUMERIC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const ASCII_PRINTABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}|;:,.<>?";
const HEX: &[u8] = b"0123456789abcdef";
const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn generate(length: usize, charset: Charset) -> String {
    let chars = match charset {
        Charset::Alphanumeric => ALPHANUMERIC,
        Charset::Ascii => ASCII_PRINTABLE,
        Charset::Hex => HEX,
        Charset::Base64 => BASE64,
    };

    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..chars.len());
            chars[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_alphanumeric() {
        let secret = generate(32, Charset::Alphanumeric);
        assert_eq!(secret.len(), 32);
        assert!(secret.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_hex() {
        let secret = generate(64, Charset::Hex);
        assert_eq!(secret.len(), 64);
        assert!(secret.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_length() {
        for len in [8, 16, 32, 64, 128] {
            let secret = generate(len, Charset::Alphanumeric);
            assert_eq!(secret.len(), len);
        }
    }

    #[test]
    fn test_charset_from_str() {
        assert!(matches!("alphanumeric".parse(), Ok(Charset::Alphanumeric)));
        assert!(matches!("hex".parse(), Ok(Charset::Hex)));
        assert!(matches!("base64".parse(), Ok(Charset::Base64)));
        assert!(matches!("ascii".parse(), Ok(Charset::Ascii)));
        assert!("invalid".parse::<Charset>().is_err());
    }
}
