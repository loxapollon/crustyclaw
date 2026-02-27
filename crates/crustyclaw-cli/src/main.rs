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
        Commands::Stop => cmd_stop(&cli.config).await?,
        Commands::Status => cmd_status(&cli.config).await?,
        Commands::Config { show } => cmd_config(&cli.config, show).await?,
        Commands::Version => cmd_version(),
        Commands::Policy {
            role,
            action,
            resource,
        } => cmd_policy(&cli.config, &role, &action, &resource).await?,
        Commands::Plugins => cmd_plugins(&cli.config).await?,
        Commands::Isolation => cmd_isolation(&cli.config).await?,
        Commands::Whoami => cmd_whoami(&cli.config).await?,
        Commands::Secrets => cmd_secrets(&cli.config).await?,
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
    let config = load_config(config_path).await?;

    // Transparent auth — authenticate the operator starting the daemon
    let session = transparent_auth(&config);
    info!(
        identity = session.identity(),
        roles = ?session.roles(),
        "Authenticated operator"
    );

    info!("Starting CrustyClaw daemon");

    let daemon = crustyclaw_core::Daemon::with_config_path(config, config_path.to_path_buf());
    daemon.run().await.map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

async fn cmd_stop(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;
    let client = ipc_client(&config);

    if !client.daemon_available() {
        eprintln!("Daemon is not running (no socket found).");
        std::process::exit(1);
    }

    let resp = client
        .stop()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to stop daemon: {e}"))?;
    println!("Daemon: {}", resp.message);
    Ok(())
}

async fn cmd_status(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;
    let client = ipc_client(&config);

    if !client.daemon_available() {
        println!("Daemon: not running");
        return Ok(());
    }

    match client.status().await {
        Ok(status) => {
            println!("Daemon: running (PID {})", status.pid);
            println!("  Version:    {} ({})", status.version, status.git_hash);
            println!("  Uptime:     {}s", status.uptime_secs);
            println!(
                "  Listen:     {}:{}",
                status.listen_addr, status.listen_port
            );
            println!(
                "  Signal:     {}",
                if status.signal_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!("  Log level:  {}", status.log_level);
            println!("  Isolation:  {}", status.isolation_backend);
            println!("  Skills:     {}", status.skills_count);
            println!("  Plugins:    {}", status.plugins_count);
        }
        Err(e) => {
            eprintln!("Failed to query daemon status: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn cmd_config(config_path: &Path, show: bool) -> Result<()> {
    let config = load_config(config_path).await?;
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

async fn cmd_policy(config_path: &Path, role: &str, action: &str, resource: &str) -> Result<()> {
    let config = load_config(config_path).await?;
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

async fn cmd_plugins(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;
    let client = ipc_client(&config);

    if !client.daemon_available() {
        println!("No plugins registered (daemon is not running).");
        println!("  Plugins are discovered at daemon startup.");
        return Ok(());
    }

    match client.plugins().await {
        Ok(resp) => {
            if resp.plugins.is_empty() {
                println!("No plugins registered.");
            } else {
                println!("Registered plugins:");
                for p in &resp.plugins {
                    println!("  {} v{} — {}", p.name, p.version, p.description);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to query plugins: {e}");
        }
    }
    Ok(())
}

async fn cmd_isolation(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;
    let iso = &config.isolation;

    // Select backend and probe availability
    let pref = match iso.backend.as_str() {
        "docker" => crustyclaw_core::isolation::BackendPreference::Docker,
        "firecracker" => crustyclaw_core::isolation::BackendPreference::Firecracker,
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
    println!("  Docker image: {}", iso.docker_image);
    println!(
        "  Credential proxy: {}",
        if iso.credential_proxy {
            "enabled"
        } else {
            "disabled"
        }
    );

    // Trust-based isolation
    if let Some(ref tier) = iso.default_trust_tier
        && let Some(trust) = crustyclaw_core::TrustTier::from_str_loose(tier)
    {
        let level = crustyclaw_core::TrustBasedSelector::required_level(trust);
        let selector = crustyclaw_core::TrustBasedSelector::new();
        let trust_backend = selector.select(trust);
        println!(
            "  Trust tier: {} → {} (backend: {})",
            trust,
            level,
            trust_backend.name()
        );
    }

    Ok(())
}

async fn cmd_whoami(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;

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

async fn cmd_secrets(config_path: &Path) -> Result<()> {
    let config = load_config(config_path).await?;

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

/// Create an IPC client from the loaded config.
fn ipc_client(config: &crustyclaw_config::AppConfig) -> crustyclaw_core::IpcClient {
    let socket_path = crustyclaw_core::ipc::server::socket_path_from_config(config);
    crustyclaw_core::IpcClient::new(socket_path)
}

async fn load_config(path: &Path) -> Result<crustyclaw_config::AppConfig> {
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        crustyclaw_config::AppConfig::load(path)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    } else {
        info!(path = %path.display(), "Config file not found, using defaults");
        Ok(crustyclaw_config::AppConfig::default())
    }
}
