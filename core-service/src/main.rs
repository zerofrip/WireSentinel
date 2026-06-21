//! WireSentinel core Windows service binary.

use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--console" || a == "-c") {
        tracing::info!("starting WireSentinel in console mode");
        return core_service::run_service(None).await;
    }

    #[cfg(windows)]
    {
        core_service::service::run_windows_service()?;
    }

    #[cfg(not(windows))]
    {
        core_service::run_service(None).await?;
    }

    Ok(())
}
