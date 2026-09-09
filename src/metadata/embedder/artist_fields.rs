//! Artist-related tag embedding: album artist, artist, composer, producer, involved people.

use std::collections::HashSet;

use lofty::tag::{
    Accessor,
    ItemKey::{AlbumArtist, Composer, Producer},
    Tag,
};

use crate::metadata::{
    config::{
        MetadataConfig,
        MetadataField::{
            AlbumArtist as ConfigAlbumArtist, Artist, Composer as ConfigComposer,
            Producer as ConfigProducer,
        },
    },
    embedder::{
        performers::{
            extract_artist_names_from_performers, extract_composers_from_performers,
            extract_producers_from_performers, is_duplicate_composer, normalize_composer_name,
        },
        push_text,
    },
    extractor::ComprehensiveMetadata,
};

/// Writes the album artist tag.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
/// * `is_flac` - Whether the output file is FLAC (affects separator and selection logic)
pub(super) fn apply_album_artist(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
    is_flac: bool,
) {
    if !config.is_enabled(ConfigAlbumArtist) {
        return;
    }
    let name = if is_flac {
        build_flac_album_artist(meta)
    } else {
        build_mp3_album_artist(meta)
    };
    if !name.is_empty() {
        push_text(tag, AlbumArtist, name);
    }
}

/// Builds album artist for FLAC: prefers conductor with main-artist role if present in performers.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// The resolved album artist name string.
fn build_flac_album_artist(meta: &ComprehensiveMetadata) -> String {
    if let Some(artists) = meta.performers.as_ref() {
        let conductor = meta.album_artists.iter().find(|a| {
            let has_main_artist_role = a
                .roles
                .as_ref()
                .is_some_and(|roles| roles.contains(&"main-artist".to_string()));
            let name_matches_conductor = a
                .name
                .as_ref()
                .is_some_and(|name| artists.contains(&format!("{name}, Conductor")));
            has_main_artist_role && name_matches_conductor
        });
        if let Some(a) = conductor {
            return a.name.clone().unwrap_or_default();
        }
    }
    meta.album_artist_name.clone().unwrap_or_default()
}

/// Builds album artist for MP3: collects all main-artist names, slash-separated.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// Slash-separated artist names string.
fn build_mp3_album_artist(meta: &ComprehensiveMetadata) -> String {
    let mut main = Vec::new();
    for a in &meta.album_artists {
        let has_role = a
            .roles
            .as_ref()
            .is_some_and(|roles| roles.contains(&"main-artist".to_string()));
        if has_role && let Some(name) = a.name.as_ref() {
            main.push(name.clone());
        }
    }
    if main.is_empty()
        && let Some(name) = meta.album_artist_name.as_ref()
    {
        main.push(name.clone());
    }
    main.join("/")
}

/// Collects artist names from the performers string into the output list.
///
/// # Arguments
///
/// * `meta` - Source metadata
/// * `names` - Output vector to append deduplicated artist names to
/// * `seen` - Set of already-seen names for deduplication
fn collect_performer_artists(
    meta: &ComprehensiveMetadata,
    names: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(performers) = meta.performers.as_ref() else {
        return;
    };
    for name in extract_artist_names_from_performers(performers, seen) {
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
}

/// Writes the artist tag by extracting names from performers, performer name, and album artists.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
/// * `is_flac` - Whether the output file is FLAC (affects separator)
pub(super) fn apply_artist(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
    is_flac: bool,
) {
    if !config.is_enabled(Artist) {
        return;
    }
    let mut names = Vec::new();
    let mut seen = HashSet::new();
    collect_performer_artists(meta, &mut names, &mut seen);
    if let Some(name) = meta.performer_name.as_ref()
        && seen.insert(name.clone())
    {
        names.push(name.clone());
    }
    for a in &meta.album_artists {
        if let Some(name) = a.name.as_ref()
            && !name.is_empty()
            && seen.insert(name.clone())
        {
            names.push(name.clone());
        }
    }
    if !names.is_empty() {
        let combined = if is_flac {
            names.join(", ")
        } else {
            names.join("/")
        };
        tag.set_artist(combined);
    }
}

/// Writes the composer tag with format-specific logic.
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
/// * `is_flac` - Whether the output file is FLAC (affects composer selection)
pub(super) fn apply_composer(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
    is_flac: bool,
) {
    if !config.is_enabled(ConfigComposer) {
        return;
    }
    let composers = if is_flac {
        build_flac_composers(meta)
    } else {
        build_mp3_composers(meta)
    };
    if !composers.is_empty() {
        push_text(tag, Composer, composers.join("/"));
    }
}

/// Builds composer list for FLAC: takes the last composer from performers.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// A vector of composer names (single entry for FLAC).
fn build_flac_composers(meta: &ComprehensiveMetadata) -> Vec<String> {
    let from_performers = meta
        .performers
        .as_ref()
        .map(|p| extract_composers_from_performers(p))
        .unwrap_or_default();
    if let Some(last) = from_performers.last()
        && last != "Various Composers"
    {
        return vec![last.clone()];
    }
    get_composer_fallback(meta)
}

/// Builds composer list for MP3: uses performers-derived composers, falling back to
/// track/album composer only when no performers have composer roles.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// A deduplicated vector of composer names.
fn build_mp3_composers(meta: &ComprehensiveMetadata) -> Vec<String> {
    let mut composers = Vec::new();
    let mut normalized = HashSet::new();
    collect_performer_composers(meta, &mut composers, &mut normalized);
    if !composers.is_empty() {
        return composers;
    }
    get_composer_fallback(meta)
}

/// Returns the composer fallback: track composer name, then album composer name.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// A vector with the first available composer name, or empty if neither exists.
fn get_composer_fallback(meta: &ComprehensiveMetadata) -> Vec<String> {
    if let Some(name) = meta.track_composer_name.as_ref()
        && name != "Various Composers"
    {
        return vec![name.clone()];
    }
    if let Some(name) = meta.album_composer_name.as_ref()
        && name != "Various Composers"
    {
        return vec![name.clone()];
    }
    Vec::new()
}

/// Collects composers from performers string into the output list with deduplication.
///
/// # Arguments
///
/// * `meta` - Source metadata
/// * `composers` - Output vector to append composer names to
/// * `normalized` - Set of normalized names for deduplication
fn collect_performer_composers(
    meta: &ComprehensiveMetadata,
    composers: &mut Vec<String>,
    normalized: &mut HashSet<String>,
) {
    let Some(performers) = meta.performers.as_ref() else {
        return;
    };
    for c in extract_composers_from_performers(performers) {
        if c != "Various Composers"
            && !is_duplicate_composer(&c, normalized)
            && normalized.insert(normalize_composer_name(&c))
        {
            composers.push(c);
        }
    }
}

/// Writes producer tags from performers string (FLAC only).
///
/// # Arguments
///
/// * `tag` - Target tag to write into
/// * `meta` - Source metadata
/// * `config` - Field toggle configuration
/// * `is_flac` - Whether the output file is FLAC (producer tags are FLAC-only)
pub(super) fn apply_producer(
    tag: &mut Tag,
    meta: &ComprehensiveMetadata,
    config: &MetadataConfig,
    is_flac: bool,
) {
    if !config.is_enabled(ConfigProducer) || !is_flac {
        return;
    }
    if let Some(performers) = meta.performers.as_ref() {
        for producer in extract_producers_from_performers(performers) {
            push_text(tag, Producer, producer);
        }
    }
}
