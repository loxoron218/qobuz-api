//! Download URL for a track at a specific quality.

/// Quality format IDs for track downloads.
pub mod quality {
    /// MP3 320kbps.
    pub const MP3_320: i32 = 5;
    /// FLAC 16-bit/44.1kHz (CD quality).
    pub const FLAC_16_44: i32 = 6;
    /// FLAC 24-bit/96kHz (Hi-Res).
    pub const FLAC_24_96: i32 = 7;
    /// FLAC 24-bit/192kHz (Hi-Res).
    pub const FLAC_24_192: i32 = 27;
}

use serde::{Deserialize, Serialize};

/// Download URL for a track at a specific quality level.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FileUrl {
    /// Track identifier.
    pub track_id: Option<i32>,
    /// Track duration in seconds.
    pub duration: Option<f64>,
    /// Download URL.
    pub url: Option<String>,
    /// Quality format ID.
    pub format_id: Option<i32>,
    /// MIME type of the audio file.
    pub mime_type: Option<String>,
    /// Sample rate in kHz.
    pub sampling_rate: Option<f64>,
    /// Bit depth.
    pub bit_depth: Option<i32>,
    /// Response status code.
    pub status: Option<i32>,
    /// Error message if applicable.
    pub message: Option<String>,
    /// Error code if applicable.
    pub code: Option<String>,
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, ensure},
        serde_json::from_str,
    };

    use crate::models::file_url::{
        FileUrl,
        quality::{FLAC_16_44, FLAC_24_96, FLAC_24_192, MP3_320},
    };

    /// Tests quality constants are distinct.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn quality_constants_are_distinct() -> Result<()> {
        ensure!(MP3_320 == 5, "MP3 constant mismatch");
        ensure!(FLAC_16_44 == 6, "FLAC 16/44 constant mismatch");
        ensure!(FLAC_24_96 == 7, "FLAC 24/96 constant mismatch");
        ensure!(FLAC_24_192 == 27, "FLAC 24/192 constant mismatch");
        Ok(())
    }

    /// Tests file URL deserializes from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    #[test]
    fn file_url_deserializes() -> Result<()> {
        let body = r#"{"url":"https://example.com/file.flac","format_id":6}"#;
        let file_url: FileUrl = from_str(body)?;
        ensure!(
            file_url.url.as_deref() == Some("https://example.com/file.flac"),
            "URL mismatch"
        );
        ensure!(file_url.format_id == Some(6), "format mismatch");
        Ok(())
    }
}
