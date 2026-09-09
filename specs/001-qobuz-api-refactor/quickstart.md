# Quickstart: Qobuz API Rust Refactor

**Feature**: `001-qobuz-api-refactor` | **Date**: 2026-04-06

## Prerequisites

- Rust toolchain (edition 2024, latest stable)
- A valid Qobuz subscription
- A `.env` file or environment variables with credentials

## Setup

### 1. Add Dependency

```toml
[dependencies]
qobuz-api = { path = "." }
tokio = { version = "1", features = ["full"] }
```

### 2. Configure Credentials

Create a `.env` file in your project root (permissions will be set to `0600` automatically):

```env
QOBUZ_EMAIL=your-email@example.com
QOBUZ_PASSWORD=your-password
```

Or use token-based authentication:

```env
QOBUZ_USER_ID=123456
QOBUZ_USER_AUTH_TOKEN=your-auth-token
```

The library automatically extracts `QOBUZ_APP_ID` and `QOBUZ_APP_SECRET` from the Qobuz web player. These are cached in `.env` for subsequent runs.

### 3. Basic Usage

```rust
use qobuz_api::{QobuzApiService, MetadataConfig};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize service (extracts app credentials from web player or .env)
    let mut service = QobuzApiService::new()?;

    // Authenticate using environment variables
    service.authenticate_with_env()?;

    // Search for albums
    let results = service.search_albums("Dark Side of the Moon", Some(5), None)?;
    if let Some(albums) = results.items {
        for album in &albums {
            println!(
                "{} - {}",
                album.title.as_deref().unwrap_or("Unknown"),
                album.artist.as_ref()
                    .and_then(|a| a.name.as_deref())
                    .unwrap_or("Unknown Artist")
            );
        }

        // Download the first result as FLAC
        if let Some(first) = albums.first() {
            let album_id = first.id.as_deref().unwrap();
            let output_dir = Path::new("./downloads");
            let config = MetadataConfig::default();

            let files = service.download_album(
                album_id,
                6,  // FLAC 16/44.1
                output_dir,
                Some(&config),
                Some(4),  // 4 concurrent downloads
            )?;

            println!("Downloaded {} tracks", files.len());
        }
    }

    Ok(())
}
```

### 4. Quality Levels

| Format ID | Quality | Format |
|-----------|---------|--------|
| 5 | MP3 320kbps | Lossy |
| 6 | FLAC 16-bit/44.1kHz | CD quality |
| 7 | FLAC 24-bit/96kHz | Hi-Res |
| 27 | FLAC 24-bit/192kHz | Hi-Res |

### 5. Search Examples

```rust
// Search all content types
let results = service.search_catalog("Miles Davis", Some(10), None)?;

// Search tracks specifically
let tracks = service.search_tracks("So What", Some(5), None)?;

// Get album details with track IDs
let album = service.get_album("sr6843", Some("track_ids"))?;
```

### 6. Favorites Management

```rust
// Add a track to favorites
service.add_user_favorites(&[12345], "track")?;

// Get favorite albums
let favorites = service.get_user_favorites("album", Some(20), None)?;

// Remove from favorites
service.delete_user_favorites(&[12345], "track")?;
```

### 7. Custom Metadata Config

```rust
let config = MetadataConfig {
    title: true,
    artist: true,
    album: true,
    cover_art: true,
    genre: false,  // Skip genre
    comment: false,
    ..MetadataConfig::default()
};

service.download_track(12345, 6, Path::new("./downloads"), Some(&config))?;
```

## Error Handling

All operations return `Result<T, QobuzApiError>`. Errors are structured and include context:

```rust
match service.download_album(id, 6, dir, None, None) {
    Ok(files) => println!("Downloaded {} tracks", files.len()),
    Err(e) => tracing::error!(error = %e, "Album download failed"),
}
```

Rate-limited requests are automatically retried (up to 3 times with exponential backoff). Credential signature errors trigger automatic credential refresh.
