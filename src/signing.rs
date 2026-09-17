//! MD5-based request signature generation for the Qobuz API.

use md5::{Digest, Md5};

/// Converts a byte digest to a lowercase hex string using a lookup table.
///
/// # Arguments
///
/// * `bytes` - Byte slice to convert
///
/// # Returns
///
/// A lowercase hexadecimal string representation.
#[must_use]
pub fn to_hex(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len().saturating_mul(2));
    for &b in bytes {
        let hi = usize::from(b >> 4);
        let lo = usize::from(b & 0x0F);
        s.push(char::from(HEX_CHARS.get(hi).copied().unwrap_or(b'0')));
        s.push(char::from(HEX_CHARS.get(lo).copied().unwrap_or(b'0')));
    }
    s
}

/// Generates a signed request hash for general API calls.
///
/// Algorithm: sort params alphabetically by key, concatenate
/// `METHOD + endpoint + key1value1...keyNvalueN + app_secret`, MD5 hash.
///
/// # Arguments
///
/// * `method` - HTTP method (e.g., `"GET"`, `"POST"`)
/// * `endpoint` - API endpoint path (e.g., `"/user/login"`)
/// * `params` - Key-value parameter pairs (will be sorted by key)
/// * `app_secret` - Application secret for signing
///
/// # Returns
///
/// Lowercase hex string of the MD5 hash.
pub fn sign_request(
    method: &str,
    endpoint: &str,
    params: &mut [(String, String)],
    app_secret: &str,
) -> String {
    params.sort_by(|a, b| a.0.cmp(&b.0));

    let mut input = String::new();
    input.push_str(method);
    input.push_str(endpoint);

    for (key, value) in params {
        input.push_str(key);
        input.push_str(value);
    }

    input.push_str(app_secret);

    let hash = Md5::digest(input.as_bytes());
    to_hex(&hash)
}

/// Generates a signed hash for track file URL requests.
///
/// Algorithm: build fixed-format string
/// `"trackgetFileUrlformat_id{fid}intentstreamtrack_id{tid}{ts}{app_secret}"`, MD5 hash.
///
/// # Arguments
///
/// * `format_id` - Quality format ID (5, 6, 7, or 27)
/// * `track_id` - Track identifier
/// * `timestamp` - Request timestamp string
/// * `app_secret` - Application secret for signing
///
/// # Returns
///
/// Lowercase hex string of the MD5 hash.
#[must_use]
pub fn sign_track_file_url(
    format_id: i32,
    track_id: i32,
    timestamp: &str,
    app_secret: &str,
) -> String {
    let input = format!(
        "trackgetFileUrlformat_id{format_id}intentstreamtrack_id{track_id}{timestamp}{app_secret}"
    );

    let hash = Md5::digest(input.as_bytes());
    to_hex(&hash)
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, ensure},
        md5::{Digest, Md5},
    };

    use crate::signing::{sign_request, sign_track_file_url, to_hex};

    fn expected_hash(input: &str) -> String {
        to_hex(&Md5::digest(input.as_bytes()))
    }

    #[test]
    fn sign_request_produces_deterministic_hash() -> Result<()> {
        let mut params = vec![
            ("b".to_string(), "2".to_string()),
            ("a".to_string(), "1".to_string()),
        ];
        let hash = sign_request("GET", "/test", &mut params, "secret");
        ensure!(hash == expected_hash("GET/testa1b2secret"), "hash mismatch");
        Ok(())
    }

    #[test]
    fn sign_request_sorts_params_alphabetically() -> Result<()> {
        let mut params1 = vec![
            ("z".to_string(), "last".to_string()),
            ("a".to_string(), "first".to_string()),
            ("m".to_string(), "middle".to_string()),
        ];
        let mut params2 = vec![
            ("a".to_string(), "first".to_string()),
            ("m".to_string(), "middle".to_string()),
            ("z".to_string(), "last".to_string()),
        ];
        let hash1 = sign_request("GET", "/ep", &mut params1, "s");
        let hash2 = sign_request("GET", "/ep", &mut params2, "s");
        ensure!(
            hash1 == hash2,
            "hashes should match regardless of input order"
        );
        Ok(())
    }

    #[test]
    fn sign_track_file_url_produces_correct_format() -> Result<()> {
        let hash = sign_track_file_url(6, 12345, "1234567890", "mysecret");
        let expected =
            expected_hash("trackgetFileUrlformat_id6intentstreamtrack_id123451234567890mysecret");
        ensure!(hash == expected, "hash mismatch");
        Ok(())
    }

    #[test]
    fn sign_request_empty_params() -> Result<()> {
        let mut params: Vec<(String, String)> = vec![];
        let hash = sign_request("POST", "/empty", &mut params, "key");
        ensure!(hash == expected_hash("POST/emptykey"), "hash mismatch");
        Ok(())
    }
}
