#![deny(unsafe_code)]

//! CrustyClaw CLI — command-line control plane.
//!
//! Provides subcommands for managing the CrustyClaw daemon, inspecting
//! configuration, evaluating security policies, and querying build info.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

/// CrustyClaw — a secure, Rust-based AI agent daemon.
#[derive(Parser)]
#[command(
    name = "crustyclaw",
    version,
    about = "CrustyClaw — secure AI agent daemon",
    long_about = "A security-first AI agent daemon written in Rust.\n\n\
        CrustyClaw routes messages between Signal and an LLM-powered skill engine,\n\
        managed via this CLI or the interactive TUI."
)]
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
        /// Show the resolved configuration as TOML.
        #[arg(long)]
        show: bool,
    },

    /// Show build version, git hash, and build profile.
    Version,

    /// Evaluate a policy access check.
    Policy {
        /// Role to check (e.g. "admin", "user").
        #[arg(long)]
        role: String,
        /// Action to check (e.g. "read", "write").
        #[arg(long)]
        action: String,
        /// Resource to check (e.g. "config", "secrets").
        #[arg(long)]
        resource: String,
    },

    /// List registered plugins (from config).
    Plugins,
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
        Commands::Version => cmd_version(),
        Commands::Policy {
            role,
            action,
            resource,
        } => cmd_policy(&cli.config, &role, &action, &resource)?,
        Commands::Plugins => cmd_plugins()?,
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
    eprintln!("Stop command not yet implemented (daemon IPC pending)");
    Ok(())
}

async fn cmd_status() -> Result<()> {
    info!("Querying CrustyClaw daemon status");
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
        println!(
            "  Daemon: {}:{}",
            config.daemon.listen_addr, config.daemon.listen_port
        );
        println!(
            "  Signal: {}",
            if config.signal.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!("  Log level: {}", config.logging.level);
        println!("  Policy rules: {}", config.policy.rules.len());
    }
    Ok(())
}

fn cmd_version() {
    println!(
        "CrustyClaw {}",
        crustyclaw_core::build_info::version_string()
    );
    println!("  Version:  {}", crustyclaw_core::build_info::VERSION);
    println!("  Git hash: {}", crustyclaw_core::build_info::GIT_HASH);
    println!("  Profile:  {}", crustyclaw_core::build_info::BUILD_PROFILE);
}

fn cmd_policy(config_path: &Path, role: &str, action: &str, resource: &str) -> Result<()> {
    let config = load_config(config_path)?;
    let mut engine = config.build_policy_engine();

    let decision = engine.evaluate(role, action, resource);
    let symbol = match decision {
        crustyclaw_config::policy::PolicyDecision::Allowed => "ALLOWED",
        crustyclaw_config::policy::PolicyDecision::Denied => "DENIED",
        crustyclaw_config::policy::PolicyDecision::NoMatch => "NO MATCH (default deny)",
    };

    println!("Policy check: role={role} action={action} resource={resource}");
    println!("  Result: {symbol}");
    println!("  Total rules: {}", engine.rule_count());

    Ok(())
}

fn cmd_plugins() -> Result<()> {
    let registry = crustyclaw_core::PluginRegistry::new();
    let names = registry.plugin_names();
    if names.is_empty() {
        println!("No plugins registered.");
        println!("  Plugins are discovered at daemon startup.");
    } else {
        println!("Registered plugins:");
        for name in names {
            println!("  - {name}");
        }
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
