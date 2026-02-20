#![deny(unsafe_code)]

//! CrustyClaw CLI — command-line control plane.
//!
//! Provides subcommands for managing the CrustyClaw daemon, inspecting
//! configuration, evaluating security policies, and querying build info.
//!
//! ## Transparent Authentication
//!
//! The CLI authenticates automatically using the OS identity of the calling
//! process (Unix UID/username). No password, token, or user interaction is
//! required. The `whoami` subcommand shows the resolved identity and roles.

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

    /// Show isolation / sandbox configuration and backend status.
    Isolation,

    /// Show current authentication identity, roles, and policy evaluation.
    ///
    /// Uses transparent local authentication — no password or token required.
    Whoami,

    /// Show configured secrets (names and sources only, never values).
    Secrets,
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
        Commands::Isolation => cmd_isolation(&cli.config)?,
        Commands::Whoami => cmd_whoami(&cli.config)?,
        Commands::Secrets => cmd_secrets(&cli.config)?,
    }

    Ok(())
}

/// Perform transparent authentication and return the authorized session.
///
/// This is called automatically by commands that need auth context.
/// The operator never sees a prompt — the OS identity is the credential.
fn transparent_auth(
    config: &crustyclaw_config::AppConfig,
) -> crustyclaw_core::auth::Session<crustyclaw_core::auth::Authorized> {
    let session = crustyclaw_core::auth::Session::new().authenticate_local();

    // Check if the config has an explicit role mapping for this user
    let identity = session.identity().to_string();
    let local = session.local_identity().cloned();

    if let Some(mapped_role) = config.auth.role_map.get(&identity) {
        // Use the explicitly configured role
        let mut roles = vec![mapped_role.clone()];
        // Also include the default role if different
        if let Some(ref li) = local {
            let default = li.default_role().to_string();
            if default != *mapped_role {
                roles.push(default);
            }
        }
        session.authorize(roles)
    } else {
        // Fall back to policy-based authorization
        let mut engine = config.build_policy_engine();
        session.authorize_with_policy(&mut engine)
    }
}

async fn cmd_start(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;

    // Transparent auth — authenticate the operator starting the daemon
    let session = transparent_auth(&config);
    info!(
        identity = session.identity(),
        roles = ?session.roles(),
        "Authenticated operator"
    );

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
        println!("  Isolation backend: {}", config.isolation.backend);
        println!("  Secrets: {} configured", config.secrets.entries.len());
        println!("  Auth mode: {}", config.auth.mode);
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

fn cmd_isolation(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;
    let iso = &config.isolation;

    // Select backend and probe availability
    let pref = match iso.backend.as_str() {
        "apple-vz" => crustyclaw_core::isolation::BackendPreference::AppleVz,
        "linux-ns" => crustyclaw_core::isolation::BackendPreference::LinuxNamespace,
        "noop" => crustyclaw_core::isolation::BackendPreference::Noop,
        _ => crustyclaw_core::isolation::BackendPreference::Auto,
    };
    let backend = crustyclaw_core::isolation::select_backend(&pref);

    println!("Isolation configuration:");
    println!("  Backend (config): {}", iso.backend);
    println!(
        "  Backend (resolved): {} (available: {})",
        backend.name(),
        backend.available()
    );
    println!(
        "  Default memory: {} MiB",
        iso.default_memory_bytes / (1024 * 1024)
    );
    println!(
        "  Default CPU fraction: {:.0}%",
        iso.default_cpu_fraction * 100.0
    );
    println!(
        "  Default timeout: {}s",
        if iso.default_timeout_secs == 0 {
            "none".to_string()
        } else {
            iso.default_timeout_secs.to_string()
        }
    );
    println!("  Default network: {}", iso.default_network);
    println!("  Max concurrent sandboxes: {}", iso.max_concurrent);

    Ok(())
}

fn cmd_whoami(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;

    // Perform transparent authentication
    let session = transparent_auth(&config);

    println!("Authentication: transparent (local OS identity)");
    println!("  Identity: {}", session.identity());
    println!("  Roles:    {:?}", session.roles());

    if let Some(local) = session.local_identity() {
        println!("  UID:      {}", local.uid);
        println!("  GID:      {}", local.gid);
        println!(
            "  Privileged: {}",
            if local.is_privileged { "yes" } else { "no" }
        );
    }

    // Show what this identity can do according to the policy engine
    let mut engine = config.build_policy_engine();
    let role = session.roles().first().map(|r| r.as_str()).unwrap_or("*");

    println!("\nPolicy evaluation (role={role}):");
    for (action, resource) in &[
        ("read", "config"),
        ("write", "config"),
        ("read", "secrets"),
        ("execute", "skills"),
        ("read", "messages"),
    ] {
        let allowed = engine.is_allowed(role, action, resource);
        let symbol = if allowed { "ALLOW" } else { "DENY" };
        println!("  {action:>8} {resource:<12} {symbol}");
    }

    Ok(())
}

fn cmd_secrets(config_path: &Path) -> Result<()> {
    let config = load_config(config_path)?;

    if config.secrets.entries.is_empty() {
        println!("No secrets configured.");
        println!("  Add secrets to [secrets.entries] in crustyclaw.toml");
        return Ok(());
    }

    println!("Secrets ({} configured):", config.secrets.entries.len());
    println!("  Staging directory: {}", config.secrets.staging_dir);
    println!();

    for entry in &config.secrets.entries {
        println!("  {}:", entry.name);
        println!("    Source:    {}", entry.source);
        println!("    Inject as: {}", entry.inject_as);
        if let Some(ref env_name) = entry.inject_env {
            println!("    Env var:   {env_name}");
        }
        if let Some(ref path) = entry.inject_path {
            println!("    File path: {path}");
        }
        if !entry.description.is_empty() {
            println!("    Note:      {}", entry.description);
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
