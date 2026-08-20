//! Metadata embedding into audio files using `lofty`.

mod artist_fields;
mod basic_fields;
mod performers;

use std::path::{Path, PathBuf};

use {
    lofty::{
        config::WriteOptions,
        file::{FileType::Flac, TaggedFileExt},
        ogg::tag::VorbisComments,
        probe::Probe,
        tag::{Tag, TagExt},
    },
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    tracing::debug,
};

use crate::{
    errors::QobuzApiError::{self, MetadataError},
    metadata::{
        config::MetadataConfig,
        embedder::{
            artist_fields::{apply_album_artist, apply_artist, apply_composer, apply_producer},
            basic_fields::{
                apply_album, apply_copyright, apply_cover_art, apply_dates, apply_disc_numbers,
                apply_flac_custom_keys, apply_genre, apply_isrc, apply_label, apply_media_type,
                apply_title, apply_track_numbers, apply_url,
            },
        },
        extractor::ComprehensiveMetadata,
    },
};

/// Embeds metadata into an audio file.
///
/// Writes tags using `lofty` with format-specific handling:
/// - FLAC: Vorbis Comments
/// - MP3: `ID3v2`
///
/// Respects `MetadataConfig` field toggles.
///
/// # Arguments
///
/// * `path` - Path to the audio file
/// * `metadata` - Metadata to embed
/// * `config` - Configuration controlling which fields to embed
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a `QobuzApiError` if metadata writing fails.
pub fn embed_metadata_in_file(
    path: &Path,
    metadata: &ComprehensiveMetadata,
    config: &MetadataConfig,
) -> Result<(), QobuzApiError> {
    debug!(path = %path.display(), "Embedding metadata");

    let mut tagged_file = Probe::open(path)
        .map_err(|e| MetadataError(format!("Probe open: {e}")))?
        .read()
        .map_err(|e| MetadataError(format!("Probe read: {e}")))?;

    let is_flac = tagged_file.file_type() == Flac;

    let tag = if let Some(t) = tagged_file.primary_tag_mut() {
        t
    } else {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
        tagged_file
            .primary_tag_mut()
            .ok_or_else(|| MetadataError("No tag created".into()))?
    };

    tag.clear();

    apply_title(tag, metadata, config);
    apply_album(tag, metadata, config);
    apply_album_artist(tag, metadata, config, is_flac);
    apply_artist(tag, metadata, config, is_flac);
    apply_composer(tag, metadata, config, is_flac);
    apply_producer(tag, metadata, config, is_flac);
    apply_label(tag, metadata, config);
    apply_genre(tag, metadata, config);
    apply_track_numbers(tag, metadata, config);
    apply_disc_numbers(tag, metadata, config);
    apply_copyright(tag, metadata, config);
    apply_isrc(tag, metadata, config);
    apply_dates(tag, metadata, config, is_flac);
    apply_url(tag, metadata, config);
    apply_media_type(tag, metadata, config);
    apply_cover_art(tag, metadata, config);

    if is_flac {
        let mut vc: VorbisComments = tag.clone().into();
        apply_flac_custom_keys(&mut vc, metadata, config);
        vc.save_to_path(path, WriteOptions::default())
            .map_err(|e| MetadataError(format!("Save failed: {e}")))?;
    } else {
        tag.save_to_path(path, WriteOptions::default())
            .map_err(|e| MetadataError(format!("Save failed: {e}")))?;
    }

    debug!(path = %path.display(), "Metadata embedded");
    Ok(())
}

/// Embeds metadata into multiple files in parallel using `rayon`.
///
/// # Arguments
///
/// * `files` - Slice of `(path, metadata)` pairs to process
/// * `config` - Configuration controlling which fields to embed
///
/// # Returns
///
/// A vector of results, one per file, in the same order as input.
#[must_use]
pub fn embed_metadata_batch(
    files: &[(PathBuf, ComprehensiveMetadata)],
    config: &MetadataConfig,
) -> Vec<Result<(), QobuzApiError>> {
    files
        .par_iter()
        .map(|(path, metadata)| embed_metadata_in_file(path, metadata, config))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{fs::write, io::Result, path::Path};

    use {
        anyhow::{Result as AnyhowResult, anyhow, ensure},
        lofty::{
            file::TaggedFileExt,
            picture::PictureType::CoverFront,
            probe::Probe,
            tag::{Accessor, Tag},
        },
        tempfile::TempDir,
    };

    use crate::metadata::{
        config::{
            MetadataConfig,
            MetadataField::{Artist, CoverArt, Title},
        },
        embedder::embed_metadata_in_file,
        extractor::ComprehensiveMetadata,
    };

    fn create_minimal_flac(path: &Path) -> Result<()> {
        let mut data = b"fLaC\x80\x00\x00\x22".to_vec();
        let streaminfo = [
            0x10u8, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0A, 0xC4, 0x40, 0xF0,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        data.extend_from_slice(&streaminfo);
        write(path, data)
    }

    fn get_tag(path: &Path) -> AnyhowResult<Tag> {
        let tagged_file = Probe::open(path)
            .map_err(|e| anyhow!("probe: {e}"))?
            .read()
            .map_err(|e| anyhow!("read: {e}"))?;
        tagged_file
            .primary_tag()
            .cloned()
            .ok_or_else(|| anyhow!("no tag"))
    }

    fn sample_metadata() -> ComprehensiveMetadata {
        ComprehensiveMetadata {
            title: Some("Test Song".into()),
            album: Some("Test Album".into()),
            performer_name: Some("Test Artist".into()),
            genre: Some("Rock".into()),
            album_release_date_original: Some("2024-01-15".into()),
            track_composer_name: Some("Composer Name".into()),
            track_number: Some(1),
            disc_number: Some(1),
            track_total: Some(10),
            disc_total: Some(1),
            isrc: Some("USXXX2400001".into()),
            copyright: Some("(C) 2024 Label".into()),
            label: Some("Test Label".into()),
            ..ComprehensiveMetadata::default()
        }
    }

    fn cover_art_metadata() -> ComprehensiveMetadata {
        ComprehensiveMetadata {
            cover_art_data: Some(vec![0xFFu8, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x00, 0x00]),
            ..sample_metadata()
        }
    }

    #[test]
    fn embed_flac_all_fields() -> AnyhowResult<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.flac");
        create_minimal_flac(&path)?;
        embed_metadata_in_file(&path, &sample_metadata(), &MetadataConfig::all())?;
        let tag = get_tag(&path)?;
        ensure!(tag.title().as_deref() == Some("Test Song"));
        ensure!(tag.artist().as_deref() == Some("Test Artist"));
        ensure!(tag.album().as_deref() == Some("Test Album"));
        ensure!(tag.genre().as_deref() == Some("Rock"));
        ensure!(tag.track() == Some(1));
        ensure!(tag.disk() == Some(1));
        Ok(())
    }

    #[test]
    fn embed_flac_selective_fields() -> AnyhowResult<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.flac");
        create_minimal_flac(&path)?;
        let mut config = MetadataConfig::default();
        config.set(Title, false);
        config.set(Artist, false);
        embed_metadata_in_file(&path, &sample_metadata(), &config)?;
        let tag = get_tag(&path)?;
        ensure!(tag.title().is_none());
        ensure!(tag.artist().is_none());
        ensure!(tag.album().as_deref() == Some("Test Album"));
        Ok(())
    }

    #[test]
    fn embed_cover_art() -> AnyhowResult<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.flac");
        create_minimal_flac(&path)?;
        embed_metadata_in_file(&path, &cover_art_metadata(), &MetadataConfig::all())?;
        let tag = get_tag(&path)?;
        ensure!(!tag.pictures().is_empty());
        ensure!(tag.pictures()[0].pic_type() == CoverFront);
        Ok(())
    }

    #[test]
    fn embed_cover_art_skipped_when_disabled() -> AnyhowResult<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.flac");
        create_minimal_flac(&path)?;
        let mut config = MetadataConfig::all();
        config.set(CoverArt, false);
        embed_metadata_in_file(&path, &cover_art_metadata(), &config)?;
        ensure!(get_tag(&path)?.pictures().is_empty());
        Ok(())
    }

    #[test]
    fn embed_title_with_version() -> AnyhowResult<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("test.flac");
        create_minimal_flac(&path)?;
        let meta = ComprehensiveMetadata {
            track_version: Some("Remastered 2015".into()),
            ..sample_metadata()
        };
        embed_metadata_in_file(&path, &meta, &MetadataConfig::all())?;
        ensure!(get_tag(&path)?.title().as_deref() == Some("Test Song (Remastered 2015)"));
        Ok(())
    }
}
