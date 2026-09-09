//! Configuration for which metadata fields to embed.

use std::collections::HashSet;

use tracing::info;

use crate::metadata::config::MetadataField::{
    Album, AlbumArtist, Artist, Comment, Composer, Copyright, CoverArt, DiscNumber, DiscTotal,
    Explicit, Genre, InvolvedPeople, Isrc, Label, MediaType, Producer, ReleaseDate, ReleaseYear,
    Title, TrackNumber, TrackTotal, Upc, Url,
};

/// Configuration controlling which metadata fields to embed in audio files.
///
/// Uses a set of `MetadataField` variants. `Default` enables all fields except `Comment`.
/// Use `is_enabled()` to check whether a field should be embedded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataConfig {
    /// Set of enabled metadata fields.
    enabled: HashSet<MetadataField>,
}

impl MetadataConfig {
    /// Creates a config with all fields enabled.
    ///
    /// # Returns
    ///
    /// A `MetadataConfig` with every `MetadataField` enabled.
    #[must_use]
    pub fn all() -> Self {
        Self {
            enabled: HashSet::from([
                Title,
                Artist,
                Album,
                AlbumArtist,
                Genre,
                ReleaseDate,
                ReleaseYear,
                Composer,
                TrackNumber,
                TrackTotal,
                DiscNumber,
                DiscTotal,
                CoverArt,
                Isrc,
                Copyright,
                Label,
                MediaType,
                Comment,
                Producer,
                InvolvedPeople,
                Explicit,
                Upc,
                Url,
            ]),
        }
    }

    /// Returns whether a specific field is enabled for embedding.
    ///
    /// # Arguments
    ///
    /// * `field` - The metadata field to check
    ///
    /// # Returns
    ///
    /// `true` if the field is enabled.
    #[must_use]
    pub fn is_enabled(&self, field: MetadataField) -> bool {
        self.enabled.contains(&field)
    }

    /// Enables or disables a specific field.
    ///
    /// # Arguments
    ///
    /// * `field` - The metadata field to toggle
    /// * `enabled` - `true` to enable, `false` to disable
    pub fn set(&mut self, field: MetadataField, enabled: bool) {
        if enabled {
            let inserted = self.enabled.insert(field);
            info!(field = ?field, inserted, "metadata field toggled");
        } else {
            let removed = self.enabled.remove(&field);
            info!(field = ?field, removed, "metadata field toggled");
        }
    }
}

impl Default for MetadataConfig {
    fn default() -> Self {
        let mut config = Self::all();
        let removed = config.enabled.remove(&Comment);
        info!(removed, "default metadata config excludes comment");
        config
    }
}

/// Metadata fields that can be embedded in audio files.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MetadataField {
    /// Track title.
    Title,
    /// Artist name.
    Artist,
    /// Album title.
    Album,
    /// Album artist.
    AlbumArtist,
    /// Genre.
    Genre,
    /// Full release date (YYYY-MM-DD).
    ReleaseDate,
    /// Release year only.
    ReleaseYear,
    /// Composer.
    Composer,
    /// Track number.
    TrackNumber,
    /// Total tracks in album.
    TrackTotal,
    /// Disc number.
    DiscNumber,
    /// Total discs in album.
    DiscTotal,
    /// Cover art image.
    CoverArt,
    /// ISRC code.
    Isrc,
    /// Copyright notice.
    Copyright,
    /// Record label.
    Label,
    /// Original media type (album, compilation, etc.).
    MediaType,
    /// Comment field.
    Comment,
    /// Producer.
    Producer,
    /// Involved people / musician credits.
    InvolvedPeople,
    /// Explicit content flag.
    Explicit,
    /// Universal Product Code.
    Upc,
    /// Commercial URL.
    Url,
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use crate::metadata::config::{Comment, MetadataConfig, Producer, Title};

    /// Tests default excludes comment.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn default_excludes_comment() -> Result<()> {
        let config = MetadataConfig::default();
        ensure!(config.is_enabled(Title));
        ensure!(!config.is_enabled(Comment));
        Ok(())
    }

    /// Tests all enables every field.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn all_enables_every_field() -> Result<()> {
        let config = MetadataConfig::all();
        ensure!(config.is_enabled(Comment));
        ensure!(config.is_enabled(Producer));
        Ok(())
    }

    /// Tests set toggles field.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn set_toggles_field() -> Result<()> {
        let mut config = MetadataConfig::default();
        ensure!(!config.is_enabled(Comment));
        config.set(Comment, true);
        ensure!(config.is_enabled(Comment));
        config.set(Comment, false);
        ensure!(!config.is_enabled(Comment));
        Ok(())
    }
}
