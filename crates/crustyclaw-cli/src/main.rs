#![deny(unsafe_code)]

//! CrustyClaw CLI — command-line control plane.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

/// CrustyClaw — a secure, Rust-based AI agent daemon.
#[derive(Parser)]
#[command(name = "crustyclaw", version, about, long_about = None)]
struct Cli {
    /// Path to configuration file.
    #[arg(short, long, default_value = "crustyclaw.toml")]
    config: PathBuf,

    /// Increase log verbosity (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the CrustyClaw daemon.
    Start,

    /// Stop a running CrustyClaw daemon.
    Stop,

    /// Show daemon status.
    Status,

    /// Validate and display configuration.
    Config {
        /// Show the resolved configuration.
        #[arg(long)]
        show: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up tracing subscriber with verbosity level
    let filter = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .init();

    match cli.command {
        Commands::Start => cmd_start(&cli.config).await?,
        Commands::Stop => cmd_stop().await?,
        Commands::Status => cmd_status().await?,
        Commands::Config { show } => cmd_config(&cli.config, show)?,
    }

    Ok(())
}

async fn cmd_start(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;
    info!("Starting CrustyClaw daemon");

    let daemon = crustyclaw_core::Daemon::new(config);
    daemon.run().await.map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

async fn cmd_stop() -> Result<()> {
    info!("Sending stop signal to CrustyClaw daemon");
    // TODO: Connect to running daemon and send shutdown signal
    eprintln!("Stop command not yet implemented (daemon IPC pending)");
    Ok(())
}

async fn cmd_status() -> Result<()> {
    info!("Querying CrustyClaw daemon status");
    // TODO: Connect to running daemon and query status
    eprintln!("Status command not yet implemented (daemon IPC pending)");
    Ok(())
}

fn cmd_config(config_path: &Path, show: bool) -> Result<()> {
    let config = load_config(config_path)?;
    if show {
        let toml_str =
            toml::to_string_pretty(&config).map_err(|e| anyhow::anyhow!("TOML error: {e}"))?;
        println!("{toml_str}");
    } else {
        println!("Configuration at '{}' is valid.", config_path.display());
    }
    Ok(())
}

fn load_config(path: &Path) -> Result<crustyclaw_config::AppConfig> {
    if path.exists() {
        crustyclaw_config::AppConfig::load(path).map_err(|e| anyhow::anyhow!(e))
    } else {
        info!(path = %path.display(), "Config file not found, using defaults");
        Ok(crustyclaw_config::AppConfig::default())
    }
}
