//! Binary entrypoint for the Qobuz API client.

use {
    anyhow::{Context, Error, Result},
    tokio::main,
    tracing_subscriber::{EnvFilter, fmt},
};

/// Application entrypoint.
#[main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|error| Error::msg(error.to_string()))
        .context("Failed to initialize tracing subscriber")?;

    Ok(())
}
