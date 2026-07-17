//! Web player credential extraction from the Qobuz JavaScript bundle.

use std::string::ToString;

use {
    base64::{Engine, engine::general_purpose::STANDARD},
    regex::Regex,
    reqwest::Client,
};

use crate::errors::QobuzApiError::{self, CredentialsError};

/// Extracts `app_id` and `app_secret` from the Qobuz web player JavaScript bundle.
///
/// The `app_id` is extracted as plain text from the production API config.
/// The `app_secret` is derived via a multi-step process: extracting `seed` and
/// `timezone` from an `initialSeed` call, locating the timezone object to find
/// `info` and `extras` fields, concatenating and truncating them, then base64
/// decoding the result.
///
/// # Returns
///
/// A tuple of `(app_id, app_secret)` extracted from the web player.
///
/// # Errors
///
/// Returns `QobuzApiError::CredentialsError` if extraction fails or `QobuzApiError::HttpError` on
/// network failure.
pub async fn extract_from_web_player() -> Result<(String, String), QobuzApiError> {
    let client = Client::builder().user_agent("Mozilla/5.0").build()?;

    let login_page = client
        .get("https://play.qobuz.com/login")
        .send()
        .await?
        .text()
        .await?;

    let bundle_url = extract_bundle_url(&login_page)?;

    let bundle_js = client.get(bundle_url).send().await?.text().await?;

    let app_id = extract_app_id_from_bundle(&bundle_js)?;
    let app_secret = extract_app_secret_from_bundle(&bundle_js)?;

    Ok((app_id, app_secret))
}

/// Extracts the bundle JavaScript URL from the login page HTML.
///
/// # Arguments
///
/// * `html` - Raw HTML of the Qobuz login page
///
/// # Returns
///
/// The full URL of the bundle JavaScript file.
fn extract_bundle_url(html: &str) -> Result<String, QobuzApiError> {
    let re = Regex::new(r#"src="(/[^"]*bundle[^"]*\.js)""#).map_err(|e| CredentialsError {
        message: format!("Invalid regex: {e}"),
    })?;

    let caps = re.captures(html).ok_or_else(|| CredentialsError {
        message: "Could not find bundle.js URL in login page".to_string(),
    })?;

    let path = caps.get(1).ok_or_else(|| CredentialsError {
        message: "Could not extract bundle.js URL from capture group".to_string(),
    })?;

    Ok(format!("https://play.qobuz.com{}", path.as_str()))
}

/// Extracts the application ID from the production API config in the bundle JS.
///
/// Matches the `production:{api:{appId:"..."` pattern.
///
/// # Arguments
///
/// * `js` - JavaScript source of the Qobuz web player bundle
///
/// # Returns
///
/// The extracted `app_id`.
fn extract_app_id_from_bundle(js: &str) -> Result<String, QobuzApiError> {
    let re =
        Regex::new(r#"production:\{api:\{appId:"([0-9]+)""#).map_err(|e| CredentialsError {
            message: format!("Invalid regex: {e}"),
        })?;

    let caps = re.captures(js).ok_or_else(|| CredentialsError {
        message: "Could not find production appId in bundle JavaScript".to_string(),
    })?;

    Ok(caps
        .get(1)
        .ok_or_else(|| CredentialsError {
            message: "Could not extract appId from capture group".to_string(),
        })?
        .as_str()
        .to_string())
}

/// Extracts the application secret from the bundle JS via multi-step base64 decoding.
///
/// The process:
/// 1. Extract `seed` and `timezone` from the `initialSeed(...)` call
/// 2. Find the timezone object by capitalized timezone name
/// 3. Extract `info` and `extras` from that timezone object
/// 4. Concatenate `seed + info + extras`, truncate the last 44 characters
/// 5. Base64-decode the result and interpret as UTF-8
///
/// # Arguments
///
/// * `js` - JavaScript source of the Qobuz web player bundle
fn extract_app_secret_from_bundle(js: &str) -> Result<String, QobuzApiError> {
    let seed_timezone_re = Regex::new(
        r#"\):[a-z]\.initialSeed\("(?P<seed>.*?)",window\.utimezone\.(?P<timezone>[a-z]+)\)"#,
    )
    .map_err(|e| CredentialsError {
        message: format!("Invalid regex for seed/timezone extraction: {e}"),
    })?;

    let seed_timezone_caps = seed_timezone_re
        .captures(js)
        .ok_or_else(|| CredentialsError {
            message: "Could not find seed and timezone in bundle JavaScript".to_string(),
        })?;

    let seed = seed_timezone_caps.name("seed").map_or("", |m| m.as_str());

    let timezone = seed_timezone_caps
        .name("timezone")
        .map_or("", |m| m.as_str());

    let title_case_timezone = capitalize_first_letter(timezone);

    let info_extras_pattern = format!(r#"name:"[^"]*/{title_case_timezone}"[^}}]*"#);

    let info_extras_re = Regex::new(&info_extras_pattern).map_err(|e| CredentialsError {
        message: format!("Invalid regex for info/extras extraction: {e}"),
    })?;

    let info_extras_caps = info_extras_re
        .captures(js)
        .ok_or_else(|| CredentialsError {
            message: "Could not find timezone object with info and extras".to_string(),
        })?;

    let timezone_object_str = info_extras_caps.get(0).map_or("", |m| m.as_str());

    let info_re = Regex::new(r#"info:"(?P<info>[^"]*)""#).map_err(|e| CredentialsError {
        message: format!("Invalid regex for info extraction: {e}"),
    })?;

    let info = info_re
        .captures(timezone_object_str)
        .and_then(|c| c.name("info"))
        .map_or("", |m| m.as_str());

    let extras_re = Regex::new(r#"extras:"(?P<extras>[^"]*)""#).map_err(|e| CredentialsError {
        message: format!("Invalid regex for extras extraction: {e}"),
    })?;

    let extras = extras_re
        .captures(timezone_object_str)
        .and_then(|c| c.name("extras"))
        .map_or("", |m| m.as_str());

    let mut base64_encoded = format!("{seed}{info}{extras}");

    if base64_encoded.len() <= 44 {
        return Err(CredentialsError {
            message: "Concatenated seed+info+extras too short to decode".to_string(),
        });
    }

    let truncate_len = base64_encoded.len() - 44;
    base64_encoded.truncate(truncate_len);

    let decoded = STANDARD
        .decode(base64_encoded)
        .map_err(|e| CredentialsError {
            message: format!("Failed to base64-decode app secret: {e}"),
        })?;

    String::from_utf8(decoded).map_err(|e| CredentialsError {
        message: format!("Decoded app secret is not valid UTF-8: {e}"),
    })
}

/// Returns the given string with its first character uppercased.
///
/// # Arguments
///
/// * `s` - Input string slice
fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, bail, ensure};

    use super::{
        capitalize_first_letter, extract_app_id_from_bundle, extract_app_secret_from_bundle,
        extract_bundle_url,
    };

    #[test]
    fn extract_bundle_url_finds_js_url() -> Result<()> {
        let html = r#"<script src="/resources/8.1.0-b019/bundle.js"></script>"#;
        let url = extract_bundle_url(html)?;
        ensure!(
            url == "https://play.qobuz.com/resources/8.1.0-b019/bundle.js",
            "url mismatch: got {url}"
        );
        Ok(())
    }

    #[test]
    fn extract_bundle_url_fails_on_missing_url() -> Result<()> {
        let html = "<html>No scripts here</html>";
        if extract_bundle_url(html).is_ok() {
            bail!("expected error for missing bundle URL");
        }
        Ok(())
    }

    #[test]
    fn extract_app_id_from_bundle_finds_production_config() -> Result<()> {
        let js = r#"integration:{api:{appId:"123",appSecret:"abc"}},production:{api:{appId:"798273057",appSecret:"05a4851e74ee47fda346f50cfdfc4f09"},braze:f}"#;
        let id = extract_app_id_from_bundle(js)?;
        ensure!(id == "798273057", "id mismatch: got {id}");
        Ok(())
    }

    #[test]
    fn extract_app_id_from_bundle_fails_on_missing_config() -> Result<()> {
        let js = r#"integration:{api:{appId:"123",appSecret:"abc"}}"#;
        if extract_app_id_from_bundle(js).is_ok() {
            bail!("expected error when production config is missing");
        }
        Ok(())
    }

    #[test]
    fn extract_app_secret_from_bundle_decodes_base64() -> Result<()> {
        // Base64("abc") = "YWJj"
        // Append 44 chars of padding, then truncate last 44 → "YWJj" → "abc"
        let js = concat!(
            r#"):e.initialSeed("#,
            r#""YW","#,
            r#"window.utimezone.mytz)"#,
            r#"other:"x",name:"/Mytz","#,
            r#"info:"Jj","#,
            r#"extras:"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA""#,
        );
        let secret = extract_app_secret_from_bundle(js)?;
        ensure!(secret == "abc", "secret mismatch: got {secret}");
        Ok(())
    }

    #[test]
    fn extract_app_secret_from_bundle_fails_on_missing_seed() -> Result<()> {
        let js = "no initial seed here";
        if extract_app_secret_from_bundle(js).is_ok() {
            bail!("expected error when seed/timezone pattern is missing");
        }
        Ok(())
    }

    #[test]
    fn capitalize_first_letter_works() -> Result<()> {
        ensure!(capitalize_first_letter("hello") == "Hello");
        ensure!(capitalize_first_letter("Hello") == "Hello");
        ensure!(capitalize_first_letter("h") == "H");
        ensure!(capitalize_first_letter("") == "");
        ensure!(capitalize_first_letter("europeparis") == "Europeparis");
        Ok(())
    }
}
