//! Credential management: `.env` file I/O, web player extraction.
//!
//! Web player extraction lives in [`web`].

pub mod web;

use std::{
    fs::{read_to_string, write},
    io::Result as IoResult,
    path::Path,
    string::ToString,
};

#[cfg(unix)]
use std::{
    fs::{Permissions, set_permissions},
    os::unix::fs::PermissionsExt,
};

use crate::errors::QobuzApiError::{self, CredentialsError};

/// Reads `QOBUZ_APP_ID` and `QOBUZ_APP_SECRET` from a `.env` file.
///
/// # Arguments
///
/// * `path` - Path to the `.env` file
///
/// # Returns
///
/// `Some((app_id, app_secret))` if both values are found, `None` otherwise.
///
/// # Errors
///
/// Returns `QobuzApiError::CredentialsError` if the file cannot be read or parsed.
pub fn load_app_credentials(path: &Path) -> Result<Option<(String, String)>, QobuzApiError> {
    if !path.exists() {
        return Ok(None);
    }

    let mut app_id = None;
    let mut app_secret = None;

    for (key, value) in parse_env_file(path).map_err(|e| CredentialsError {
        message: format!("Failed to read .env file: {e}"),
    })? {
        match key.as_str() {
            "QOBUZ_APP_ID" => app_id = Some(value),
            "QOBUZ_APP_SECRET" => app_secret = Some(value),
            _ => {}
        }
    }

    match (app_id, app_secret) {
        (Some(id), Some(secret)) => Ok(Some((id, secret))),
        _ => Ok(None),
    }
}

/// Writes `QOBUZ_APP_ID` and `QOBUZ_APP_SECRET` to a `.env` file with `0600` permissions.
///
/// # Arguments
///
/// * `path` - Path to the `.env` file
/// * `app_id` - Application ID to store
/// * `app_secret` - Application secret to store
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns `QobuzApiError::IoError` if the file cannot be written or permissions set.
pub fn save_app_credentials(
    path: &Path,
    app_id: &str,
    app_secret: &str,
) -> Result<(), QobuzApiError> {
    let existing = if path.exists() {
        read_to_string(path)?
    } else {
        String::new()
    };

    let lines: Vec<&str> = existing.lines().collect();
    let mut updated_lines: Vec<String> = lines.iter().map(ToString::to_string).collect();
    let mut id_written = false;
    let mut secret_written = false;

    for line in &mut updated_lines {
        if line.starts_with("QOBUZ_APP_ID=") || line == "# QOBUZ_APP_ID=" {
            *line = format!("QOBUZ_APP_ID={app_id}");
            id_written = true;
        } else if line.starts_with("QOBUZ_APP_SECRET=") || line == "# QOBUZ_APP_SECRET=" {
            *line = format!("QOBUZ_APP_SECRET={app_secret}");
            secret_written = true;
        } else {
            // preserve other lines unchanged
        }
    }

    if !id_written {
        updated_lines.push(format!("QOBUZ_APP_ID={app_id}"));
    }
    if !secret_written {
        updated_lines.push(format!("QOBUZ_APP_SECRET={app_secret}"));
    }

    let content = updated_lines.join("\n");
    write(path, content)?;

    set_file_permissions(path)?;

    Ok(())
}

/// Sets file permissions to owner-only read/write (`0600`) on Unix systems.
///
/// # Arguments
///
/// * `path` - Path to the file
///
/// # Returns
///
/// `Ok(())` on success.
fn set_file_permissions(path: &Path) -> Result<(), QobuzApiError> {
    #[cfg(unix)]
    {
        set_permissions(path, Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Parses a single line of a `.env` file into a key-value pair.
///
/// Returns `None` for blank lines and comment lines (starting with `#`).
fn parse_env_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim().to_string();
    let value = value.trim();

    let value = unquote(value).to_string();

    Some((key, value))
}

/// Strips surrounding single or double quotes from a value string.
fn unquote(s: &str) -> &str {
    let len = s.len();
    if len < 2 {
        return s;
    }

    let bytes = s.as_bytes();
    if (bytes[0] == b'"' && bytes[len - 1] == b'"')
        || (bytes[0] == b'\'' && bytes[len - 1] == b'\'')
    {
        &s[1..len - 1]
    } else {
        s
    }
}

/// Parses a `.env` file and returns all key-value pairs.
///
/// Handles comments, blank lines, and optional quoting on values.
///
/// # Arguments
///
/// * `path` - Path to the `.env` file
///
/// # Returns
///
/// An `io::Result` with the parsed key-value pairs.
///
/// # Errors
///
/// Returns an I/O error if the file cannot be read.
pub fn parse_env_file(path: &Path) -> IoResult<Vec<(String, String)>> {
    let content = read_to_string(path)?;
    Ok(content.lines().filter_map(parse_env_line).collect())
}

#[cfg(test)]
mod tests {
    use std::{fs::read_to_string, io::Write, path::Path};

    use {
        anyhow::{Result, anyhow, ensure},
        tempfile::{NamedTempFile, tempdir},
    };

    use crate::credentials::{load_app_credentials, save_app_credentials};

    fn create_temp_env(contents: &str) -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        write!(file, "{contents}")?;
        Ok(file)
    }

    #[test]
    fn load_app_credentials_reads_existing_file() -> Result<()> {
        let file = create_temp_env("QOBUZ_APP_ID=12345\nQOBUZ_APP_SECRET=secret123\n")?;
        let result = load_app_credentials(file.path())?;
        let (id, secret) = result.ok_or_else(|| anyhow!("expected Some"))?;
        ensure!(id == "12345", "id mismatch");
        ensure!(secret == "secret123", "secret mismatch");
        Ok(())
    }

    #[test]
    fn load_app_credentials_returns_none_for_missing_fields() -> Result<()> {
        let file = create_temp_env("QOBUZ_APP_ID=12345\n")?;
        let result = load_app_credentials(file.path())?;
        ensure!(result.is_none(), "expected None when fields missing");
        Ok(())
    }

    #[test]
    fn load_app_credentials_returns_none_for_nonexistent_file() -> Result<()> {
        let result = load_app_credentials(Path::new("/nonexistent/.env"))?;
        ensure!(result.is_none(), "expected None for nonexistent file");
        Ok(())
    }

    #[test]
    fn save_app_credentials_creates_new_file() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join(".env");

        save_app_credentials(&path, "app123", "secret456")?;

        let content = read_to_string(&path)?;
        ensure!(content.contains("QOBUZ_APP_ID=app123"), "missing app_id");
        ensure!(
            content.contains("QOBUZ_APP_SECRET=secret456"),
            "missing app_secret"
        );
        Ok(())
    }

    #[test]
    fn save_app_credentials_updates_existing_values() -> Result<()> {
        let file = create_temp_env("QOBUZ_APP_ID=old\nQOBUZ_APP_SECRET=oldsecret\n")?;
        save_app_credentials(file.path(), "new", "newsecret")?;

        let content = read_to_string(file.path())?;
        ensure!(
            content.contains("QOBUZ_APP_ID=new"),
            "should contain new id"
        );
        ensure!(
            content.contains("QOBUZ_APP_SECRET=newsecret"),
            "should contain new secret"
        );
        ensure!(!content.contains("old"), "should not contain old values");
        Ok(())
    }

    #[test]
    fn save_app_credentials_replaces_commented_placeholders() -> Result<()> {
        let file = create_temp_env("# QOBUZ_APP_ID=\n# QOBUZ_APP_SECRET=\nQOBUZ_USER_ID=123\n")?;
        save_app_credentials(file.path(), "app_id_val", "secret_val")?;

        let content = read_to_string(file.path())?;
        ensure!(
            content.contains("QOBUZ_APP_ID=app_id_val"),
            "should contain app_id"
        );
        ensure!(
            content.contains("QOBUZ_APP_SECRET=secret_val"),
            "should contain app_secret"
        );
        ensure!(
            !content.contains("# QOBUZ_APP_ID="),
            "should not contain commented placeholder"
        );
        ensure!(
            !content.contains("# QOBUZ_APP_SECRET="),
            "should not contain commented placeholder"
        );
        ensure!(
            content.contains("QOBUZ_USER_ID=123"),
            "should preserve other lines"
        );
        Ok(())
    }
}
