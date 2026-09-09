//! `ExifTool` extraction and parsing for metadata tests.

use std::{fs::write, path::Path, process::Command};

use anyhow::{Result, anyhow, bail};

use crate::metadata_test::ExifEntry;

/// Extracts metadata from an audio file using `ExifTool` and saves the raw output to disk.
///
/// # Arguments
///
/// * `file_path` - Path to the audio file
/// * `output_path` - Path to write the raw `ExifTool` output
///
/// # Returns
///
/// Parsed `ExifTool` entries.
///
/// # Errors
///
/// Returns an error if extraction fails or the output cannot be written.
pub fn extract_and_save_metadata(file_path: &Path, output_path: &Path) -> Result<Vec<ExifEntry>> {
    let entries = extract_metadata_exiftool(file_path)?;
    let output = Command::new("exiftool")
        .arg("-G1")
        .arg(file_path)
        .output()
        .map_err(|e| anyhow!("exiftool: {e}"))?;
    write(
        output_path,
        String::from_utf8_lossy(&output.stdout).as_bytes(),
    )?;
    Ok(entries)
}

/// Extracts metadata from an audio file using `exiftool -G1`.
///
/// # Arguments
///
/// * `file_path` - Path to the audio file
///
/// # Returns
///
/// Parsed `ExifTool` entries.
///
/// # Errors
///
/// Returns an error if `exiftool` fails to run or exits unsuccessfully.
pub fn extract_metadata_exiftool(file_path: &Path) -> Result<Vec<ExifEntry>> {
    let output = Command::new("exiftool")
        .arg("-G1")
        .arg(file_path)
        .output()
        .map_err(|e| anyhow!("exiftool failed to start: {e}"))?;
    if !output.status.success() {
        bail!(
            "exiftool exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(parse_exiftool_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

/// Parses raw `exiftool -G1` output into a list of entries.
///
/// # Arguments
///
/// * `content` - Raw stdout from `exiftool -G1`
///
/// # Returns
///
/// A vector of parsed `ExifEntry` instances.
#[must_use]
pub fn parse_exiftool_output(content: &str) -> Vec<ExifEntry> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((before, after)) = trimmed.split_once(':') else {
            continue;
        };
        let value = after.trim();
        let field = before.rsplit(']').next().map_or("", str::trim);
        if field.is_empty() || value.is_empty() {
            continue;
        }
        entries.push(ExifEntry {
            field: field.to_string(),
            value: value.to_string(),
        });
    }
    entries
}
