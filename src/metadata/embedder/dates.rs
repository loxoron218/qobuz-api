//! Primary date resolution for metadata embedding.

use crate::metadata::extractor::ComprehensiveMetadata;

/// Resolves the primary date using priority: album download > album original > track original >
/// `released_at` timestamp.
///
/// # Arguments
///
/// * `meta` - Source metadata
///
/// # Returns
///
/// A tuple of `(full_date_string, year)`, either of which may be `None`.
pub(super) fn determine_primary_date(
    meta: &ComprehensiveMetadata,
) -> (Option<String>, Option<u32>) {
    if let Some(d) = meta.album_release_date_download.as_ref() {
        return (Some(d.clone()), parse_year(d));
    }
    if let Some(d) = meta.album_release_date_original.as_ref() {
        return (Some(d.clone()), parse_year(d));
    }
    if let Some(d) = meta.track_release_date_original.as_ref() {
        return (Some(d.clone()), parse_year(d));
    }
    if let Some(ts) = meta.released_at {
        return timestamp_to_date_and_year(ts);
    }
    (None, None)
}

/// Parses a 4-digit year from a date string.
///
/// # Arguments
///
/// * `date` - Date string in `YYYY-MM-DD` or similar format
///
/// # Returns
///
/// The year as `Some(u32)`, or `None` if parsing fails.
fn parse_year(date: &str) -> Option<u32> {
    let year_str = date.split('-').next()?;
    let Ok(year) = year_str.parse::<u32>() else {
        return None;
    };
    Some(year)
}

/// Converts a Unix timestamp to a date string and year.
///
/// # Arguments
///
/// * `timestamp` - Unix timestamp in seconds
///
/// # Returns
///
/// A tuple of `(formatted_date_string, year)`.
fn timestamp_to_date_and_year(timestamp: i64) -> (Option<String>, Option<u32>) {
    let days = timestamp.div_euclid(86400);
    let z = days.saturating_add(719_468);
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = doe
        .saturating_sub(doe / 1460)
        .saturating_add(doe / 36_524)
        .saturating_sub(doe / 146_096)
        / 365;
    let y = yoe.saturating_add(era.saturating_mul(400));
    let doy = doe.saturating_sub(
        (365_i64)
            .saturating_mul(yoe)
            .saturating_add(yoe / 4)
            .saturating_sub(yoe / 100),
    );
    let mp = (5_i64).saturating_mul(doy).saturating_add(2) / 153;
    let d = doy
        .saturating_sub((153_i64).saturating_mul(mp).saturating_add(2) / 5)
        .saturating_add(1);
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let y = if m <= 2 { y.saturating_add(1) } else { y };
    (
        Some(format!("{y:04}-{m:02}-{d:02}")),
        Some(y.try_into().unwrap_or(0)),
    )
}
