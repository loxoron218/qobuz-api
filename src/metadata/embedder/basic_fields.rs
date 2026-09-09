//! Basic metadata field embedding: title, album, genre, dates, ISRC, copyright, cover art, etc.

use lofty::{
    ogg::tag::VorbisComments,
    picture::{MimeType::Jpeg, Picture, PictureType::CoverFront},
    tag::{
        Accessor,
        ItemKey::{
            CommercialInformationUrl, CopyrightMessage, Isrc, Label, OriginalMediaType,
            RecordingDate, ReleaseDate, Year,
        },
        Tag,
    },
};

use crate::metadata::{
    config::{
        MetadataConfig,
        MetadataField::{
            Album, Copyright, CoverArt, DiscNumber, DiscTotal, Explicit, Genre, InvolvedPeople,
            Isrc as FieldIsrc, Label as FieldLabel, MediaType, ReleaseDate as FieldReleaseDate,
            ReleaseYear, Title, TrackNumber, TrackTotal, Upc, Url as FieldUrl,
        },
    },
    embedder::{dates::determine_primary_date, push_text},
    extractor::ComprehensiveMetadata,
};

/// Writes the track title, appending version if present.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_title(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(Title) {
        return;
    }
    if let Some(title) = meta.title.as_ref() {
        let full = match &meta.track_version {
            Some(v) if !v.is_empty() => format!("{title} ({v})"),
            _ => title.clone(),
        };
        tag.set_title(full);
    }
}

/// Writes the album title, appending version if present.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_album(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(Album) {
        return;
    }
    if let Some(album_title) = meta.album.as_ref() {
        let full = match &meta.album_version {
            Some(v) if !v.is_empty() => format!("{album_title} ({v})"),
            _ => album_title.clone(),
        };
        tag.set_album(full);
    }
}

/// Writes the label / publisher tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_label(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(FieldLabel) {
        return;
    }
    if let Some(label) = meta.label.as_ref() {
        push_text(tag, Label, label.clone());
    }
}

/// Writes the genre tag if enabled.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_genre(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(Genre) {
        return;
    }
    if let Some(genre) = meta.genre.as_ref() {
        tag.set_genre(genre.clone());
    }
}

/// Writes track number and track total tags.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_track_numbers(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if config.is_enabled(TrackNumber)
        && let Some(n) = meta.track_number
    {
        tag.set_track(n.cast_unsigned());
    }
    if config.is_enabled(TrackTotal)
        && let Some(n) = meta.track_total
    {
        tag.set_track_total(n.cast_unsigned());
    }
}

/// Writes disc number and disc total tags.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_disc_numbers(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if config.is_enabled(DiscNumber)
        && let Some(n) = meta.disc_number
    {
        tag.set_disk(n.cast_unsigned());
    }
    if config.is_enabled(DiscTotal)
        && let Some(n) = meta.disc_total
    {
        tag.set_disk_total(n.cast_unsigned());
    }
}

/// Writes the copyright message tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_copyright(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if !config.is_enabled(Copyright) {
        return;
    }
    if let Some(copyright) = meta.copyright.as_ref() {
        push_text(tag, CopyrightMessage, copyright.clone());
    }
}

/// Writes the ISRC tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_isrc(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(FieldIsrc) {
        return;
    }
    if let Some(isrc) = meta.isrc.as_ref() {
        push_text(tag, Isrc, isrc.clone());
    }
}

/// Writes release date and year tags with format-specific keys.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
/// * `is_flac` - Whether the output file is FLAC (affects tag key selection)
pub(super) fn apply_dates(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
    is_flac: bool,
) {
    let (date_full, year) = determine_primary_date(meta);
    if config.is_enabled(ReleaseYear)
        && let Some(y) = year
    {
        let key = if is_flac { Year } else { RecordingDate };
        push_text(tag, key, y.to_string());
    }
    if !config.is_enabled(FieldReleaseDate) {
        return;
    }
    if let Some(date_str) = date_full.as_ref() {
        let key = if is_flac { RecordingDate } else { ReleaseDate };
        push_text(tag, key, date_str.clone());
    }
}

/// Writes the commercial information URL tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_url(tag: &mut Tag, meta: &ComprehensiveMetadata, config: &MetadataConfig) {
    if !config.is_enabled(FieldUrl) {
        return;
    }
    if let Some(url) = meta.product_url.as_ref() {
        let full = if url.starts_with("http") {
            url.clone()
        } else {
            format!("https://www.qobuz.com{url}")
        };
        push_text(tag, CommercialInformationUrl, full);
    }
}

/// Writes the original media type tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_media_type(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if !config.is_enabled(MediaType) {
        return;
    }
    let media = meta.release_type.as_ref().or(meta.product_type.as_ref());
    if let Some(mt) = media {
        push_text(tag, OriginalMediaType, mt.clone());
    }
}

/// Writes embedded cover art.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata containing cover art binary data
/// * `config` - Field toggle configuration
pub(super) fn apply_cover_art(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if !config.is_enabled(CoverArt) {
        return;
    }
    if let Some(data) = &meta.cover_art_data {
        let picture = Picture::unchecked(data.clone())
            .pic_type(CoverFront)
            .mime_type(Jpeg)
            .build();
        tag.push_picture(picture);
    }
}

/// Writes FLAC-specific `VorbisComment` keys that have no `lofty` `ItemKey` mapping.
///
/// These keys (`INVOLVEDPEOPLE`, `ITUNESADVISORY`, `ORGANIZATION`, `UPC`, `URL`)
/// are written by the C# reference implementation but silently dropped by lofty's
/// generic `Tag` → `VorbisComments` conversion.
///
/// # Arguments
///
/// * `vc` - Target `VorbisComments` tag
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
pub(super) fn apply_flac_custom_keys(
    vc: &mut VorbisComments,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
) {
    if config.is_enabled(InvolvedPeople)
        && let Some(performers) = meta.performers.as_ref()
        && !performers.is_empty()
    {
        let formatted = performers
            .replace("\r\n", ". ")
            .replace('\r', ". ")
            .trim()
            .to_string();
        vc.push("INVOLVEDPEOPLE".to_string(), formatted);
    }
    if config.is_enabled(Explicit)
        && let Some(warning) = meta.parental_warning
    {
        let val = if warning { "1" } else { "0" };
        vc.push("ITUNESADVISORY".to_string(), val.to_string());
    }
    if config.is_enabled(FieldLabel)
        && let Some(label) = meta.label.as_ref()
    {
        vc.push("ORGANIZATION".to_string(), label.clone());
    }
    if config.is_enabled(Upc)
        && let Some(upc) = meta.upc.as_ref()
    {
        vc.push("UPC".to_string(), upc.clone());
    }
    if config.is_enabled(FieldUrl)
        && let Some(url) = meta.product_url.as_ref()
    {
        let full = if url.starts_with("http") {
            url.clone()
        } else {
            format!("https://www.qobuz.com{url}")
        };
        vc.push("URL".to_string(), full);
    }
}
