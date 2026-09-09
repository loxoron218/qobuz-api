//! Metadata field comparison logic for integration tests.

use std::collections::{HashMap, HashSet};

use crate::verification::{
    DIRECTORY_FILENAME_IGNORED, DURATION_IGNORED, ExifEntry, FILE_DATE_TIME_IGNORED,
    FILE_SIZE_IGNORED,
    FieldDifference::{self, Differs, OnlyInCSharp, OnlyInRust},
    LAME_IGNORED, PICTURE_IGNORED, VERSION_IGNORED,
};

/// Compares `ExifTool` metadata entries between Rust and C# outputs.
///
/// Filters out known-ignored fields and returns both meaningful differences
/// and ignored field counts.
///
/// # Arguments
///
/// * `rust_entries` - Metadata entries from the Rust-embedded file
/// * `csharp_entries` - Metadata entries from the C# reference file
/// * `is_mp3` - Whether the format is MP3 (enables additional LAME ignored fields)
///
/// # Returns
///
/// A tuple of (differences, `ignored_field_counts`).
#[must_use]
pub fn compare_exif_metadata(
    rust_entries: &[ExifEntry],
    csharp_entries: &[ExifEntry],
    is_mp3: bool,
) -> (Vec<FieldDifference>, Vec<(String, String)>) {
    let rust_map: HashMap<&str, &str> = rust_entries
        .iter()
        .map(|e| (e.field.as_str(), e.value.as_str()))
        .collect();
    let csharp_map: HashMap<&str, &str> = csharp_entries
        .iter()
        .map(|e| (e.field.as_str(), e.value.as_str()))
        .collect();
    let mut diffs = Vec::new();
    let mut ignored = Vec::new();
    let all_keys: HashSet<&&str> = rust_map.keys().chain(csharp_map.keys()).collect();
    for &key in &all_keys {
        let rust_val = rust_map.get(key).copied();
        let csharp_val = csharp_map.get(key).copied();
        let is_ignored = is_ignored_field(key, is_mp3);
        if is_ignored && rust_val != csharp_val {
            ignored.push((key.to_string(), "ignored (always different)".to_string()));
        }
        if is_ignored {
            continue;
        }
        if let Some(diff) = diff_field(key, rust_val, csharp_val) {
            diffs.push(diff);
        }
    }
    (diffs, ignored)
}

/// Compares Rust and C# field values and returns a `FieldDifference` if they differ.
///
/// # Arguments
///
/// * `key` - The field name being compared
/// * `rust_val` - Value from the Rust implementation (may be `None`)
/// * `csharp_val` - Value from the C# reference (may be `None`)
///
/// # Returns
///
/// `Some(FieldDifference)` if the values differ or one is missing, `None` if identical.
fn diff_field(
    key: &str,
    rust_val: Option<&str>,
    csharp_val: Option<&str>,
) -> Option<FieldDifference> {
    match (rust_val, csharp_val) {
        (Some(rv), Some(cv)) if !values_equivalent(rv, cv) => Some(Differs {
            field: key.to_string(),
            rust_value: rv.to_string(),
            csharp_value: cv.to_string(),
        }),
        (Some(rv), None) => Some(OnlyInRust {
            field: key.to_string(),
            value: rv.to_string(),
        }),
        (None, Some(cv)) => Some(OnlyInCSharp {
            field: key.to_string(),
            value: cv.to_string(),
        }),
        _ => None,
    }
}

/// Returns whether a metadata field should be ignored during comparison.
///
/// # Arguments
///
/// * `field` - The `ExifTool` field name to check
/// * `is_mp3` - Whether the format is MP3 (enables additional LAME ignored fields)
///
/// # Returns
///
/// `true` if the field is in an ignored category.
#[must_use]
pub fn is_ignored_field(field: &str, is_mp3: bool) -> bool {
    let f = |list: &[&str]| list.contains(&field);
    f(FILE_DATE_TIME_IGNORED)
        || f(FILE_SIZE_IGNORED)
        || f(DIRECTORY_FILENAME_IGNORED)
        || f(PICTURE_IGNORED)
        || f(DURATION_IGNORED)
        || f(VERSION_IGNORED)
        || (is_mp3 && f(LAME_IGNORED))
}

/// Groups two related fields (e.g., "Musician Credits" / "Involved People") in the diff list.
///
/// If both fields exist and their values match, the pair is moved to the ignored list
/// under the `grouped_name`. Otherwise, they are merged under `grouped_name` as a diff.
///
/// # Arguments
///
/// * `diffs` - Vector of field differences
/// * `ignored` - Vector of ignored field differences
/// * `field_a` - First field name to check
/// * `field_b` - Second field name to check
/// * `grouped_name` - Name to use for the grouped result
pub fn group_field_pair(
    diffs: &mut Vec<FieldDifference>,
    ignored: &mut Vec<(String, String)>,
    field_a: &str,
    field_b: &str,
    grouped_name: &str,
) {
    let a_diff = extract_diff_by_field(diffs, field_a);
    let b_diff = extract_diff_by_field(diffs, field_b);

    match (a_diff, b_diff) {
        (Some(a), Some(b)) => {
            let val_a = diff_value(&a);
            let val_b = diff_value(&b);
            if values_equivalent(&val_a, &val_b) {
                ignored.push((
                    grouped_name.to_string(),
                    "ignored (matching values)".to_string(),
                ));
            } else {
                diffs.push(Differs {
                    field: grouped_name.to_string(),
                    rust_value: val_a,
                    csharp_value: val_b,
                });
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            ignored.push((
                grouped_name.to_string(),
                "ignored (only in one implementation)".to_string(),
            ));
        }
        (None, None) => {}
    }
}

/// Extracts and removes a field difference from the list by field name.
///
/// # Arguments
///
/// * `diffs` - Vector of field differences to search
/// * `field` - The field name to find
///
/// # Returns
///
/// `Some(FieldDifference)` if found and removed, `None` otherwise.
fn extract_diff_by_field(diffs: &mut Vec<FieldDifference>, field: &str) -> Option<FieldDifference> {
    let idx = diffs.iter().position(|d| diff_field_name(d) == field)?;
    Some(diffs.remove(idx))
}

/// Extracts the field name from a `FieldDifference`.
///
/// # Arguments
///
/// * `diff` - The field difference to extract from
///
/// # Returns
///
/// The field name as a string slice.
fn diff_field_name(diff: &FieldDifference) -> &str {
    match diff {
        Differs { field, .. } | OnlyInRust { field, .. } | OnlyInCSharp { field, .. } => field,
    }
}

/// Extracts the Rust-side value from a `FieldDifference`.
///
/// # Arguments
///
/// * `diff` - The field difference to extract from
///
/// # Returns
///
/// The Rust value as a `String`.
fn diff_value(diff: &FieldDifference) -> String {
    match diff {
        Differs { rust_value, .. } => rust_value.clone(),
        OnlyInRust { value, .. } | OnlyInCSharp { value, .. } => value.clone(),
    }
}

/// Checks if two metadata values are equivalent, accounting for reordering of
/// dash-separated credits (e.g., "A - B" vs "B - A").
///
/// # Arguments
///
/// * `a` - First value to compare
/// * `b` - Second value to compare
///
/// # Returns
///
/// `true` if the values are identical or contain the same credits in any order.
fn values_equivalent(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let norm_a = normalize_whitespace(a);
    let norm_b = normalize_whitespace(b);
    if norm_a == norm_b {
        return true;
    }
    let mut parts_a: Vec<String> = norm_a.split(" - ").map(normalize_credit).collect();
    let mut parts_b: Vec<String> = norm_b.split(" - ").map(normalize_credit).collect();
    parts_a.sort_unstable();
    parts_b.sort_unstable();
    parts_a == parts_b
}

/// Collapses consecutive whitespace into a single space and trims.
///
/// # Arguments
///
/// * `s` - The string to normalize
///
/// # Returns
///
/// A string with consecutive whitespace collapsed into single spaces and trimmed.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Normalizes a single credit entry by sorting its comma-separated roles
/// while keeping the name (first element) in place.
///
/// # Arguments
///
/// * `credit` - A credit string like "Name, Role1, Role2"
///
/// # Returns
///
/// A normalized string with sorted roles, e.g. "Name, Role2, Role1".
fn normalize_credit(credit: &str) -> String {
    let mut parts: Vec<&str> = credit.split(", ").collect();
    if parts.len() > 1 {
        let name = parts.remove(0);
        parts.sort_unstable();
        parts.insert(0, name);
    }
    parts.join(", ")
}
