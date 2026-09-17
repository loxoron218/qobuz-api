//! Binary entrypoint for the Qobuz API client.

use {
    anyhow::{Result, anyhow},
    tokio::main,
    tracing_subscriber::{EnvFilter, fmt},
};

/// Application entrypoint.
#[main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow!(e))?;

    Ok(())
}
