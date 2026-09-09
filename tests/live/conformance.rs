//! Integration tests comparing Rust metadata embedding against C# reference implementation.
//!
//! Downloads 20 diverse tracks from Qobuz in both FLAC and MP3 formats, embeds metadata
//! using the Rust implementation, extracts tags with `exiftool -G1`, and compares the
//! results against known-good C# reference files.
//!
//! **Requires `exiftool` on `$PATH`.**
//!
//! Setup: copy `.env.example` to `.env` and fill in your Qobuz credentials, then:
//!
//! Run with `cargo test --test live --features live-tests`.

#[cfg(test)]
mod tests {
    use std::{fs::read_to_string, path::Path};

    use {
        anyhow::{Result, anyhow, ensure},
        tracing::info,
    };

    use qobuz_api::api::service::QobuzApiService;

    use crate::{
        create_authenticated_service, init_logging,
        verification::{
            FieldDifference::{self, Differs, OnlyInCSharp, OnlyInRust},
            ReportSummary, TestTrack, build_file_header,
            comparison::{compare_exif_metadata, group_field_pair},
            csharp_metadata_dir,
            exiftool::{extract_and_save_metadata, parse_exiftool_output},
            metadata_dir,
            report::{generate_comparison_report, generate_format_report},
            staging::{download_test_track, ensure_directories, save_track_json},
            test_tracks, track_filename_base,
        },
    };

    const FLAC_FORMAT: FormatInfo = FormatInfo {
        format_id: 27,
        label: "FLAC",
        dir_name: "flac",
    };

    const MP3_FORMAT: FormatInfo = FormatInfo {
        format_id: 5,
        label: "MP3",
        dir_name: "mp3",
    };

    struct FormatInfo {
        format_id: i32,
        label: &'static str,
        dir_name: &'static str,
    }

    fn diff_field(diff: &FieldDifference) -> String {
        match diff {
            Differs { field, .. } | OnlyInRust { field, .. } | OnlyInCSharp { field, .. } => {
                field.clone()
            }
        }
    }

    #[test]
    fn init() {
        init_logging();
    }

    fn process_format(
        service: &mut QobuzApiService,
        track: &TestTrack,
        fmt: &FormatInfo,
    ) -> Result<ReportSummary> {
        let file_path = download_test_track(service, track, fmt.format_id)?;

        let base = track_filename_base(track, fmt.label);
        let meta_path = metadata_dir(fmt.dir_name).join(format!("{base}.txt"));

        let rust_entries = extract_and_save_metadata(&file_path, &meta_path)?;

        let csharp_path =
            csharp_metadata_dir(fmt.dir_name).join(format!("{}.txt", track.csharp_base));

        if !csharp_path.exists() {
            return Err(anyhow!("C# reference missing: {}", csharp_path.display()));
        }

        let csharp_content = read_to_string(&csharp_path)?;
        let csharp_entries = parse_exiftool_output(&csharp_content);

        let is_mp3 = fmt.label == "MP3";
        let (mut diffs, mut ignored) =
            compare_exif_metadata(&rust_entries, &csharp_entries, is_mp3);

        group_field_pair(
            &mut diffs,
            &mut ignored,
            "Musician Credits",
            "Involved People",
            "Musician Credits / Involved People",
        );

        if !is_mp3 {
            group_field_pair(
                &mut diffs,
                &mut ignored,
                "Media",
                "Mediatype",
                "Media / Mediatype",
            );
        }

        if !diffs.is_empty() {
            generate_comparison_report(track, fmt.label, &diffs)?;
        }

        let mut summary = ReportSummary::default();
        let header = build_file_header(track, fmt.label);
        for diff in &diffs {
            let field_name = diff_field(diff);
            summary
                .differences
                .entry(field_name)
                .or_default()
                .push((header.clone(), diff.clone()));
        }
        for (field, _) in &ignored {
            let count = summary.ignored_counts.entry(field.clone()).or_default();
            *count = count.saturating_add(1);
        }

        Ok(summary)
    }

    #[test]
    fn live_download_all_tracks_and_compare() -> Result<()> {
        ensure_directories()?;

        let mut service = create_authenticated_service()?;

        let mut flac_summaries: Vec<(&TestTrack, ReportSummary)> = Vec::new();
        let mut mp3_summaries: Vec<(&TestTrack, ReportSummary)> = Vec::new();

        for track in test_tracks() {
            save_track_json(&service, track)?;

            let flac_summary = process_format(&mut service, track, &FLAC_FORMAT)?;
            flac_summaries.push((track, flac_summary));

            let mp3_summary = process_format(&mut service, track, &MP3_FORMAT)?;
            mp3_summaries.push((track, mp3_summary));
        }

        generate_format_report("flac", &flac_summaries)?;
        generate_format_report("mp3", &mp3_summaries)?;

        Ok(())
    }

    #[test]
    fn live_verify_reports_exist() -> Result<()> {
        let flac_report = Path::new("metadata_tests/flac_report.md");
        let mp3_report = Path::new("metadata_tests/mp3_report.md");

        if !flac_report.exists() || !mp3_report.exists() {
            info!("Reports not found - run live_download_all_tracks_and_compare first");
            return Ok(());
        }

        let flac_content = read_to_string(flac_report)?;
        ensure!(
            flac_content.starts_with("# Metadata Comparison Report"),
            "FLAC report should have correct header"
        );

        let mp3_content = read_to_string(mp3_report)?;
        ensure!(
            mp3_content.starts_with("# Metadata Comparison Report"),
            "MP3 report should have correct header"
        );

        info!("Reports verified successfully");
        Ok(())
    }
}
