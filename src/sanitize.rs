//! Cross-platform filename sanitization.

/// Sanitizes a string for use as a file or directory name.
///
/// Replaces characters that are invalid in filenames on Windows, macOS, and Linux
/// with underscores, then trims leading/trailing whitespace and dots.
///
/// # Arguments
///
/// * `name` - The raw filename string to sanitize
///
/// # Returns
///
/// A sanitized filename string. Returns `"unnamed"` if the result would be empty.
#[must_use]
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'..='\x1F' => '_',
            _ => c,
        })
        .collect();

    let trimmed = sanitized.trim().trim_matches('.').trim_matches(' ');

    if trimmed.is_empty() {
        return "unnamed".to_string();
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use crate::sanitize::sanitize_filename;

    #[test]
    fn sanitize_replaces_invalid_characters() -> Result<()> {
        ensure!(sanitize_filename("hello/world") == "hello_world");
        ensure!(sanitize_filename("a:b*c?d") == "a_b_c_d");
        Ok(())
    }

    #[test]
    fn sanitize_handles_windows_invalid_chars() -> Result<()> {
        ensure!(sanitize_filename("file<>|.txt") == "file___.txt");
        Ok(())
    }

    #[test]
    fn sanitize_strips_control_characters() -> Result<()> {
        ensure!(sanitize_filename("hello\tworld\n") == "hello_world_");
        Ok(())
    }

    #[test]
    fn sanitize_trims_whitespace_and_dots() -> Result<()> {
        ensure!(sanitize_filename("  ..hello..  ") == "hello");
        Ok(())
    }

    #[test]
    fn sanitize_returns_unnamed_for_empty_string() -> Result<()> {
        ensure!(sanitize_filename("") == "unnamed");
        ensure!(sanitize_filename("...") == "unnamed");
        ensure!(sanitize_filename("   ") == "unnamed");
        Ok(())
    }

    #[test]
    fn sanitize_preserves_valid_filename() -> Result<()> {
        ensure!(sanitize_filename("track 01 - Title.flac") == "track 01 - Title.flac");
        ensure!(sanitize_filename("Album Name") == "Album Name");
        Ok(())
    }
}
