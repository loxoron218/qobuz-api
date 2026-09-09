//! Helpers for metadata comparison integration tests.

pub mod comparison;
pub mod exiftool;
pub mod report;
pub mod staging;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use qobuz_api::sanitize::sanitize_filename;

/// Constructs a [`TestTrack`] from artist, title, track ID, and C# base name.
macro_rules! track {
    ($artist:expr, $title:expr, $id:expr, $csharp:expr) => {
        TestTrack {
            artist: $artist,
            title: $title,
            track_id: $id,
            csharp_base: $csharp,
        }
    };
}

/// Metadata fields related to file name and directory paths (always differ, ignored).
pub const DIRECTORY_FILENAME_IGNORED: &[&str] = &["File Name", "Directory"];

/// Metadata fields related to duration and sample counts (encoding-dependent, ignored).
pub const DURATION_IGNORED: &[&str] = &["Duration", "Total Samples"];

/// Metadata fields related to file system timestamps (always differ, ignored).
pub const FILE_DATE_TIME_IGNORED: &[&str] = &[
    "File Modification Date/Time",
    "File Access Date/Time",
    "File Inode Change Date/Time",
];

/// Metadata fields related to file and ID3 tag sizes (always differ, ignored).
pub const FILE_SIZE_IGNORED: &[&str] = &["File Size", "ID3 Size"];

/// LAME encoder-specific metadata fields (MP3-only, encoding-dependent, ignored).
pub const LAME_IGNORED: &[&str] = &[
    "Encoder",
    "Lame Bitrate",
    "Lame Low Pass Filter",
    "Lame Method",
    "Lame Quality",
    "Lame Stereo Mode",
    "Lame VBR Quality",
    "MS Stereo",
];
/// Root directory for metadata test artifacts.
pub const METADATA_TEST_DIR: &str = "metadata_tests";

/// Picture dimension metadata fields (always differ due to resizing, ignored).
pub const PICTURE_IGNORED: &[&str] = &["Picture Bits Per Pixel", "Picture Height", "Picture Width"];

/// `ExifTool` version field (tool-dependent, ignored).
pub const VERSION_IGNORED: &[&str] = &["ExifTool Version Number"];

/// Canonical set of tracks used for metadata comparison testing.
static TEST_TRACKS: [TestTrack; 20] = [
    track!("Adele", "Hello", 28_014_963, "Adele - Hello"),
    track!(
        "Daft Punk",
        "Around the World",
        1_065_482,
        "Daft Punk - Around the World"
    ),
    track!(
        "Glenn Gould",
        "Aria _ Sarabande",
        26_890_072,
        "Glenn Gould - Aria _ Sarabande (Remastered)"
    ),
    track!(
        "Herbie Hancock",
        "Chameleon",
        37_905,
        "Herbie Hancock - Chameleon (Album Version)"
    ),
    track!(
        "Kendrick Lamar",
        "BLOOD.",
        40_128_300,
        "Kendrick Lamar - BLOOD"
    ),
    track!(
        "Madonna",
        "Like a Virgin",
        6_004_880,
        "Madonna - Like a Virgin"
    ),
    track!(
        "Miles Davis",
        "So What",
        13_176_083,
        "Miles Davis - So What"
    ),
    track!(
        "Nirvana",
        "Smells Like Teen Spirit",
        14_158_543,
        "Nirvana - Smells Like Teen Spirit (Album Version)"
    ),
    track!(
        "Staatskapelle Dresden",
        "Symphony No. 40 in G Minor, K. 550: I. Molto Allegro",
        162_673_929,
        "Staatskapelle Dresden - Symphony No. 40 in G Minor, K. 550_ I. Molto Allegro (Remastered)"
    ),
    track!(
        "The Beatles",
        "Hey Jude",
        235_803_319,
        "The Beatles - Hey Jude (Remastered 2015)"
    ),
    track!(
        "Johnny Cash",
        "Hurt",
        33_839_468,
        "Johnny Cash - Hurt (Album Version)"
    ),
    track!(
        "Stevie Wonder",
        "Superstition",
        16_012_386,
        "Stevie Wonder - Superstition (Album Version)"
    ),
    track!(
        "Bob Marley & The Wailers",
        "Three Little Birds",
        52_323_780,
        "Bob Marley & The Wailers - Three Little Birds"
    ),
    track!(
        "B.B. King",
        "The Thrill Is Gone",
        52_371_130,
        "B.B. King - The Thrill Is Gone"
    ),
    track!(
        "Metallica",
        "Enter Sandman",
        123_452_387,
        "Metallica - Enter Sandman (Remastered 2021)"
    ),
    track!(
        "Bob Dylan",
        "Like a Rolling Stone",
        15_215_741,
        "Bob Dylan - Like a Rolling Stone"
    ),
    track!(
        "Donna Summer",
        "I Feel Love",
        52_482_680,
        "Donna Summer - I Feel Love"
    ),
    track!(
        "Sex Pistols",
        "Anarchy In The UK",
        262_708_929,
        "Sex Pistols - Anarchy In The UK"
    ),
    track!(
        "Buena Vista Social Club",
        "Chan Chan",
        387_387_265,
        "Buena Vista Social Club - Chan Chan (Remastered 2021)"
    ),
    track!("Hans Zimmer", "Time", 2_206_801, "Hans Zimmer - Time"),
];

/// A single key-value entry parsed from `exiftool -G1` output.
#[derive(Debug)]
pub struct ExifEntry {
    /// `ExifTool` field name (e.g., "ID3v2.3:Album").
    pub field: String,
    /// Field value as a string.
    pub value: String,
}

/// Describes a difference between Rust and C# metadata for a single field.
#[derive(Clone, Debug)]
pub enum FieldDifference {
    /// Field exists in both but values differ.
    Differs {
        /// Field name.
        field: String,
        /// Value embedded by Rust implementation.
        rust_value: String,
        /// Value embedded by C# reference implementation.
        csharp_value: String,
    },
    /// Field exists only in the Rust output.
    OnlyInRust {
        /// Field name.
        field: String,
        /// Value in Rust output.
        value: String,
    },
    /// Field exists only in the C# output.
    OnlyInCSharp {
        /// Field name.
        field: String,
        /// Value in C# output.
        value: String,
    },
}

/// Aggregated comparison results across all tracks for one format.
#[derive(Debug, Default)]
pub struct ReportSummary {
    /// Differences grouped by field name, each with file header and diff details.
    pub differences: HashMap<String, Vec<(String, FieldDifference)>>,
    /// Count of ignored differences per field name.
    pub ignored_counts: HashMap<String, usize>,
}

/// Descriptor for a test track used in metadata comparison.
#[derive(Clone, Copy, Debug)]
pub struct TestTrack {
    /// Primary artist name.
    pub artist: &'static str,
    /// Track title.
    pub title: &'static str,
    /// Qobuz track ID.
    pub track_id: i32,
    /// Base filename of the C# reference metadata file.
    pub csharp_base: &'static str,
}

/// Returns the canonical set of tracks used for metadata comparison testing.
///
/// # Returns
///
/// A static reference to the array of test tracks.
#[must_use]
pub fn test_tracks() -> &'static [TestTrack] {
    &TEST_TRACKS
}

/// Builds a file header string for diff reports comparing Rust vs C# output.
///
/// # Arguments
///
/// * `track` - The test track descriptor
/// * `format_label` - Format label (e.g., "FLAC", "MP3")
///
/// # Returns
///
/// A formatted header string with full paths.
#[must_use]
pub fn build_file_header(track: &TestTrack, format_label: &str) -> String {
    let format_lower = format_label.to_lowercase();
    let rust_base = track_filename_base(track, format_label);
    format!(
        "@/qobuz-api-rust-refactor/metadata_tests/metadata/C#-songs/{format_lower}/{}.txt vs \
         @/qobuz-api-rust-refactor/metadata_tests/metadata/{format_lower}/{rust_base}.txt",
        track.csharp_base,
    )
}

/// Returns the path to the C# reference metadata directory for a given format.
///
/// # Arguments
///
/// * `format` - Format subdirectory name (e.g., "flac", "mp3")
///
/// # Returns
///
/// A `PathBuf` pointing to the C# reference directory.
#[must_use]
pub fn csharp_metadata_dir(format: &str) -> PathBuf {
    Path::new(METADATA_TEST_DIR)
        .join("metadata")
        .join("C#-songs")
        .join(format)
}

/// Returns the path to the downloads directory for a given format.
///
/// # Arguments
///
/// * `format` - Format subdirectory name (e.g., "flac", "mp3")
///
/// # Returns
///
/// A `PathBuf` pointing to the downloads directory.
#[must_use]
pub fn downloads_dir(format: &str) -> PathBuf {
    Path::new(METADATA_TEST_DIR).join("downloads").join(format)
}

/// Returns the path to the JSON metadata directory.
///
/// # Returns
///
/// A `PathBuf` pointing to the JSON output directory.
#[must_use]
pub fn json_dir() -> PathBuf {
    Path::new(METADATA_TEST_DIR).join("metadata").join("json")
}

/// Returns the path to the Rust metadata output directory for a given format.
///
/// # Arguments
///
/// * `format` - Format subdirectory name (e.g., "flac", "mp3")
///
/// # Returns
///
/// A `PathBuf` pointing to the metadata output directory.
#[must_use]
pub fn metadata_dir(format: &str) -> PathBuf {
    Path::new(METADATA_TEST_DIR).join("metadata").join(format)
}

/// Returns the path to the reports output directory.
///
/// # Returns
///
/// A `PathBuf` pointing to the reports directory.
#[must_use]
pub fn reports_dir() -> PathBuf {
    Path::new(METADATA_TEST_DIR).join("reports")
}

/// Builds a sanitized base filename for a test track and format.
///
/// # Arguments
///
/// * `track` - The test track descriptor
/// * `format_label` - Format label (e.g., "FLAC", "MP3")
///
/// # Returns
///
/// A sanitized filename string in the form `{artist} - {title} - {format} - {id}`.
#[must_use]
pub fn track_filename_base(track: &TestTrack, format_label: &str) -> String {
    sanitize_filename(&format!(
        "{} - {} - {format_label} - {}",
        track.artist, track.title, track.track_id
    ))
}
