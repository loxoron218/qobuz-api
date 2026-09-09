//! Performer string parsing and composer deduplication helpers.

use std::collections::HashSet;

/// Parses a performers string into `(person_name, roles)` pairs.
///
/// # Arguments
///
/// * `performers_str` - Raw performers string from the API (e.g., `"Name, Role - Other, Role2"`)
///
/// # Returns
///
/// A vector of `(person_name, role_list)` tuples.
pub(super) fn parse_performers(performers_str: &str) -> Vec<(&str, Vec<&str>)> {
    performers_str
        .split(" - ")
        .filter_map(|group| {
            let group = group.trim();
            let mut parts: Vec<&str> = group.split(',').map(str::trim).collect();
            if parts.is_empty() {
                return None;
            }
            let person_name = parts.remove(0).trim();
            Some((person_name, parts))
        })
        .collect()
}

/// Extracts artist names from the performers string, filtering by relevant roles.
///
/// # Arguments
///
/// * `performers_str` - Raw performers string from the API
/// * `existing` - Set of already-known names to skip
///
/// # Returns
///
/// A vector of artist name strings matching performance roles.
pub(super) fn extract_artist_names_from_performers(
    performers_str: &str,
    existing: &HashSet<String>,
) -> Vec<String> {
    let mut names = Vec::new();
    for (person_name, roles) in parse_performers(performers_str) {
        let has_role = roles.iter().any(|role| {
            *role == "Performer" || role.contains("MainArtist") || role.contains("FeaturedArtist")
        });
        if has_role && !existing.contains(person_name) && !names.contains(&person_name.to_string())
        {
            names.push(person_name.to_string());
        }
    }
    names
}

/// Extracts composer names from the performers string.
///
/// # Arguments
///
/// * `performers_str` - Raw performers string from the API
///
/// # Returns
///
/// A deduplicated vector of composer name strings.
pub(super) fn extract_composers_from_performers(performers_str: &str) -> Vec<String> {
    let mut composers = Vec::new();
    for (person_name, roles) in parse_performers(performers_str) {
        let is_composer = roles
            .iter()
            .any(|r| r.contains("Composer") || r.contains("Lyricist"));
        if is_composer && !composers.contains(&person_name.to_string()) {
            composers.push(person_name.to_string());
        }
    }
    composers
}

/// Extracts producer names from the performers string.
///
/// # Arguments
///
/// * `performers_str` - Raw performers string from the API
///
/// # Returns
///
/// A vector of producer name strings.
pub(super) fn extract_producers_from_performers(performers_str: &str) -> Vec<String> {
    let mut producers = Vec::new();
    for (person_name, roles) in parse_performers(performers_str) {
        if roles.contains(&"Producer") {
            producers.push(person_name.to_string());
        }
    }
    producers
}

/// Normalizes a composer name for comparison purposes.
///
/// # Arguments
///
/// * `name` - The raw composer name
///
/// # Returns
///
/// A lowercased, punctuation-normalized version of the name.
pub(super) fn normalize_composer_name(name: &str) -> String {
    name.to_lowercase()
        .trim()
        .replace(['.', ','], "")
        .replace('-', " ")
        .replace("  ", " ")
        .trim()
        .to_string()
}

/// Checks if a composer name is a duplicate of an existing one.
///
/// # Arguments
///
/// * `name` - The composer name to check
/// * `existing` - Set of already-normalized composer names
///
/// # Returns
///
/// `true` if the name matches an existing entry after normalization.
pub(super) fn is_duplicate_composer(name: &str, existing: &HashSet<String>) -> bool {
    let normalized = normalize_composer_name(name);
    if existing.contains(&normalized) {
        return true;
    }
    for e in existing {
        if e.contains(&normalized) || normalized.contains(e) {
            return true;
        }
    }
    false
}
