//! Report generation for metadata comparison tests.

use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    fs::write,
    path::Path,
};

use {anyhow::Result, qobuz_api::sanitize::sanitize_filename, tracing::info};

use crate::verification::{
    DIRECTORY_FILENAME_IGNORED, DURATION_IGNORED, FILE_DATE_TIME_IGNORED, FILE_SIZE_IGNORED,
    FieldDifference::{self, Differs, OnlyInCSharp, OnlyInRust},
    LAME_IGNORED, METADATA_TEST_DIR, PICTURE_IGNORED, ReportSummary, TestTrack, VERSION_IGNORED,
    reports_dir, track_filename_base,
};

/// Writes a single field difference to a markdown string.
///
/// # Arguments
///
/// * `md` - Target markdown string to append to
/// * `file_header` - Header identifying the file pair being compared
/// * `diff` - The field difference to write
///
/// # Returns
///
/// `Ok(())` on success, or a formatting error.
fn write_field_diff(md: &mut String, file_header: &str, diff: &FieldDifference) -> Result<()> {
    writeln!(md, "## {file_header}")?;
    match diff {
        Differs {
            field: f,
            rust_value,
            csharp_value,
        } => {
            write!(
                md,
                "Field '{f}' differs:\n  Rust: {rust_value}\n  C#:   {csharp_value}\n\n"
            )?;
        }
        OnlyInRust { field: f, value } => {
            write!(md, "Field '{f}' exists in Rust but not in C#: {value}\n\n")?;
        }
        OnlyInCSharp { field: f, value } => {
            write!(md, "Field '{f}' exists in C# but not in Rust: {value}\n\n")?;
        }
    }
    Ok(())
}

/// Generates a per-track comparison report listing differences between Rust and C# outputs.
///
/// # Arguments
///
/// * `track` - The test track descriptor
/// * `format_label` - Format label (e.g., "FLAC", "MP3")
/// * `diffs` - Differences found between Rust and C# metadata
///
/// # Returns
///
/// `Ok(())` on success, or an I/O / formatting error.
///
/// # Errors
///
/// Returns an error if the report cannot be written.
pub fn generate_comparison_report(
    track: &TestTrack,
    format_label: &str,
    diffs: &[FieldDifference],
) -> Result<()> {
    let rust_base = track_filename_base(track, format_label);
    let report_name = format!(
        "{artist}_{title}_{format_label}_{id}_vs_{csharp}.txt",
        artist = sanitize_filename(track.artist),
        title = sanitize_filename(track.title),
        id = track.track_id,
        csharp = sanitize_filename(track.csharp_base),
    );
    let report_path = reports_dir().join(&report_name);
    let mut content = format!(
        "Differences between {rust_base}.txt and {}.txt:\n",
        track.csharp_base
    );
    for diff in diffs {
        match diff {
            Differs {
                field,
                rust_value,
                csharp_value,
            } => {
                write!(
                    content,
                    "Field '{field}' differs:\n  Rust: {rust_value}\n  C#:   {csharp_value}\n"
                )?;
            }
            OnlyInRust { field, value } => {
                writeln!(
                    content,
                    "Field '{field}' exists in Rust but not in C#: {value}"
                )?;
            }
            OnlyInCSharp { field, value } => {
                writeln!(
                    content,
                    "Field '{field}' exists in C# but not in Rust: {value}"
                )?;
            }
        }
    }
    write(&report_path, content)?;
    info!(path = %report_path.display(), "Comparison report written");
    Ok(())
}

/// Generates an aggregated markdown report for all tracks in one format.
///
/// # Arguments
///
/// * `format_label` - Format label (e.g., "flac", "mp3")
/// * `summaries` - Per-track comparison summaries
///
/// # Returns
///
/// `Ok(())` on success, or an I/O / formatting error.
///
/// # Errors
///
/// Returns an error if the report cannot be written.
pub fn generate_format_report(
    format_label: &str,
    summaries: &[(&TestTrack, ReportSummary)],
) -> Result<()> {
    let filename = format!("{format_label}_report.md");
    let path = Path::new(METADATA_TEST_DIR).join(&filename);
    let total_diff_fields = summaries
        .iter()
        .map(|(_, s)| s.differences.len())
        .sum::<usize>();
    let total_diffs = summaries
        .iter()
        .map(|(_, s)| s.differences.values().map(Vec::len).sum::<usize>())
        .sum::<usize>();
    let total_ignored: usize = summaries
        .iter()
        .map(|(_, s)| s.ignored_counts.values().sum::<usize>())
        .sum();
    let is_mp3 = format_label == "mp3";
    let mut md = format!(
        "# Metadata Comparison Report\n\n## Summary\n- Total fields with differences: \
         {total_diff_fields}\n- Total {format_label} metadata differences found: {total_diffs}\n- \
         Ignored differences (always different): {total_ignored}\n\n"
    );
    md.push_str(
        "## Note on Ignored Fields\n\nThe following fields were intentionally ignored in the \
         comparison as they are always different and irrelevant to the actual metadata quality:\n",
    );
    let agg: HashMap<String, usize> = summaries
        .iter()
        .flat_map(|(_, s)| s.ignored_counts.iter())
        .fold(HashMap::new(), |mut acc, (k, v)| {
            let count = acc.entry(k.clone()).or_default();
            *count = count.saturating_add(*v);
            acc
        });
    write_grouped_ignored_fields(&mut md, &agg, is_mp3)?;
    md.push_str(
        "\nThese differences occur due to file processing, storage locations, and encoding \
         differences but do not reflect actual metadata discrepancies.\n\n",
    );
    let mut all_diffs: HashMap<String, Vec<(String, FieldDifference)>> = HashMap::new();
    for (_, summary) in summaries {
        for (field, diffs) in &summary.differences {
            all_diffs
                .entry(field.clone())
                .or_default()
                .extend(diffs.clone());
        }
    }
    if all_diffs.is_empty() {
        write!(md, "No differences found for {format_label} format.\n\n")?;
    } else {
        write_field_sections(&mut md, all_diffs)?;
    }
    write(&path, md)?;
    info!(path = %path.display(), "Format report generated");
    Ok(())
}

/// Writes grouped ignored-field categories to the report markdown.
///
/// # Arguments
///
/// * `md` - Target markdown string
/// * `agg` - Aggregated ignored-field counts
/// * `is_mp3` - Whether the format is MP3 (includes LAME categories)
///
/// # Returns
///
/// `Ok(())` on success, or a formatting error.
fn write_grouped_ignored_fields(
    md: &mut String,
    agg: &HashMap<String, usize>,
    is_mp3: bool,
) -> Result<()> {
    let grouped_categories: Vec<(&str, &[&str])> = if is_mp3 {
        vec![
            ("ExifTool Tags", VERSION_IGNORED),
            ("Directory and File Name Tags", DIRECTORY_FILENAME_IGNORED),
            ("Duration and Total Samples Tags", DURATION_IGNORED),
            ("File Date/Time Tags", FILE_DATE_TIME_IGNORED),
            ("ID3 and File Size Tags", FILE_SIZE_IGNORED),
            ("LAME Tags", LAME_IGNORED),
            ("Picture Tags", PICTURE_IGNORED),
        ]
    } else {
        vec![
            ("ExifTool Tags", VERSION_IGNORED),
            ("Directory and File Name Tags", DIRECTORY_FILENAME_IGNORED),
            ("Duration and Total Samples Tags", DURATION_IGNORED),
            ("File Date/Time Tags", FILE_DATE_TIME_IGNORED),
            ("ID3 and File Size Tags", FILE_SIZE_IGNORED),
            ("Picture Tags", PICTURE_IGNORED),
        ]
    };

    let mut standalone: Vec<(&str, &usize)> = Vec::new();
    let mut category_fields: HashSet<&str> = HashSet::new();
    for (_, fields) in &grouped_categories {
        category_fields.extend(fields.iter().copied());
    }

    for (field, count) in agg {
        if !category_fields.contains(field.as_str()) {
            standalone.push((field.as_str(), count));
        }
    }
    standalone.sort_by(|a, b| a.0.cmp(b.0));
    for (field, count) in &standalone {
        writeln!(md, "- {field}: {count}")?;
    }

    for (category_name, fields) in &grouped_categories {
        let category_entries: Vec<(&str, usize)> = fields
            .iter()
            .filter_map(|f| agg.get(*f).map(|c| (*f, *c)))
            .collect();
        if category_entries.is_empty() {
            continue;
        }
        writeln!(md, "- {category_name}:")?;
        let mut sorted = category_entries;
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (field, count) in &sorted {
            writeln!(md, "  -> {field}: {count}")?;
        }
    }

    Ok(())
}

/// Writes per-field diff sections sorted alphabetically.
///
/// # Arguments
///
/// * `md` - Target markdown string
/// * `all_diffs` - Differences grouped by field name
///
/// # Returns
///
/// `Ok(())` on success, or a formatting error.
fn write_field_sections(
    md: &mut String,
    all_diffs: HashMap<String, Vec<(String, FieldDifference)>>,
) -> Result<()> {
    let mut sorted: Vec<_> = all_diffs.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (i, (field, diffs)) in sorted.iter().enumerate() {
        writeln!(md, "#==================================================")?;
        writeln!(
            md,
            "# {}. Field: {field}: {} Cases",
            i.saturating_add(1),
            diffs.len()
        )?;
        write!(
            md,
            "#==================================================\n\n"
        )?;
        for (file_header, diff) in diffs {
            write_field_diff(md, file_header, diff)?;
        }
    }
    Ok(())
}
