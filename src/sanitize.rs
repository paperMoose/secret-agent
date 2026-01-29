use base64::Engine;
use std::collections::HashMap;

/// Sanitize output by replacing secret values with redacted placeholders
pub fn sanitize(output: &str, secrets: &HashMap<String, String>) -> String {
    let mut result = output.to_owned();

    for (name, value) in secrets {
        if value.is_empty() {
            continue;
        }

        // Direct match
        result = result.replace(value, &format!("[REDACTED:{}]", name));

        // Base64 encoded
        let b64_standard = base64::engine::general_purpose::STANDARD.encode(value);
        if !b64_standard.is_empty() {
            result = result.replace(&b64_standard, &format!("[REDACTED:{}:base64]", name));
        }

        // Base64 URL-safe encoded
        let b64_url = base64::engine::general_purpose::URL_SAFE.encode(value);
        if !b64_url.is_empty() && b64_url != b64_standard {
            result = result.replace(&b64_url, &format!("[REDACTED:{}:base64url]", name));
        }

        // URL encoded
        let url_encoded = urlencoding::encode(value);
        if url_encoded != value.as_str() {
            result = result.replace(url_encoded.as_ref(), &format!("[REDACTED:{}:urlencoded]", name));
        }
    }

    result
}

/// Sanitize bytes, returning sanitized string
pub fn sanitize_bytes(output: &[u8], secrets: &HashMap<String, String>) -> String {
    let output_str = String::from_utf8_lossy(output);
    sanitize(&output_str, secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets() -> HashMap<String, String> {
        let mut s = HashMap::new();
        s.insert("API_KEY".to_string(), "sk-12345".to_string());
        s.insert("PASSWORD".to_string(), "hunter2".to_string());
        s
    }

    #[test]
    fn test_sanitize_direct_match() {
        let output = "Connecting with token sk-12345...";
        let result = sanitize(output, &secrets());
        assert_eq!(result, "Connecting with token [REDACTED:API_KEY]...");
    }

    #[test]
    fn test_sanitize_multiple_occurrences() {
        let output = "key=sk-12345, again: sk-12345";
        let result = sanitize(output, &secrets());
        assert_eq!(
            result,
            "key=[REDACTED:API_KEY], again: [REDACTED:API_KEY]"
        );
    }

    #[test]
    fn test_sanitize_multiple_secrets() {
        let output = "user:hunter2 key:sk-12345";
        let result = sanitize(output, &secrets());
        assert!(result.contains("[REDACTED:PASSWORD]"));
        assert!(result.contains("[REDACTED:API_KEY]"));
    }

    #[test]
    fn test_sanitize_base64() {
        let secrets = {
            let mut s = HashMap::new();
            s.insert("SECRET".to_string(), "hello".to_string());
            s
        };
        // "hello" in base64 is "aGVsbG8="
        let output = "encoded: aGVsbG8=";
        let result = sanitize(output, &secrets);
        assert_eq!(result, "encoded: [REDACTED:SECRET:base64]");
    }

    #[test]
    fn test_sanitize_url_encoded() {
        let secrets = {
            let mut s = HashMap::new();
            s.insert("SECRET".to_string(), "hello world".to_string());
            s
        };
        // "hello world" url-encoded is "hello%20world"
        let output = "param=hello%20world";
        let result = sanitize(output, &secrets);
        assert_eq!(result, "param=[REDACTED:SECRET:urlencoded]");
    }

    #[test]
    fn test_sanitize_no_match() {
        let output = "nothing secret here";
        let result = sanitize(output, &secrets());
        assert_eq!(result, "nothing secret here");
    }

    #[test]
    fn test_sanitize_empty_secret() {
        let mut s = HashMap::new();
        s.insert("EMPTY".to_string(), "".to_string());
        let output = "test output";
        let result = sanitize(output, &s);
        assert_eq!(result, "test output");
    }
}
