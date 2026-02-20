#![deny(unsafe_code)]

//! Secrets management for CrustyClaw.
//!
//! Provides a [`SecretStore`] that holds named secrets in memory with
//! automatic zeroization on drop. Secrets can be loaded from:
//!
//! - TOML config (`[secrets]` section)
//! - Environment variables (`CRUSTYCLAW_SECRET_<NAME>`)
//! - Files (one secret per file, path referenced in config)
//!
//! Secrets are injected into sandbox containers via two mechanisms:
//!
//! - **Environment injection:** secret value set as an env var inside the container
//! - **File injection:** secret value written to a tmpfs-backed file, mounted read-only
//!
//! ## Security Properties
//!
//! - All secret values implement `Zeroize` and are cleared on drop.
//! - Secret values are redacted in `Debug` output (shown as `[REDACTED]`).
//! - File-injected secrets use restrictive permissions (0o400).
//! - The store never logs or displays secret values.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use zeroize::Zeroize;

/// A single secret value with automatic zeroization.
#[derive(Clone)]
pub struct SecretValue {
    /// The raw secret bytes.
    inner: String,
}

impl SecretValue {
    /// Create a new secret from a string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            inner: value.into(),
        }
    }

    /// Get the secret value as a string slice.
    ///
    /// Use sparingly — prefer injection methods over direct access.
    pub fn expose(&self) -> &str {
        &self.inner
    }

    /// Get the secret value length (without exposing the value).
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the secret value is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretValue")
            .field("inner", &"[REDACTED]")
            .field("len", &self.inner.len())
            .finish()
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}

/// How a secret should be injected into a container.
#[derive(Debug, Clone)]
pub enum InjectionMethod {
    /// Inject as an environment variable with the given name.
    ///
    /// Example: `InjectionMethod::Env("API_KEY".into())`
    /// → container sees `API_KEY=<secret_value>`
    Env(String),

    /// Inject as a read-only file at the given guest path.
    ///
    /// Example: `InjectionMethod::File("/run/secrets/api_key".into())`
    /// → container sees a file at `/run/secrets/api_key` containing the secret
    File(PathBuf),

    /// Inject as both an env var and a file.
    Both {
        /// Environment variable name.
        env_name: String,
        /// File path inside the container.
        file_path: PathBuf,
    },
}

/// A named secret with its injection configuration.
#[derive(Debug, Clone)]
pub struct SecretEntry {
    /// The secret name (used as the key in the store).
    pub name: String,
    /// The secret value (zeroized on drop).
    pub value: SecretValue,
    /// How this secret should be injected into containers.
    pub injection: InjectionMethod,
    /// Optional description for operator reference.
    pub description: String,
}

/// The source from which a secret was loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretSource {
    /// Loaded from the TOML config file.
    Config,
    /// Loaded from an environment variable.
    Environment(String),
    /// Loaded from a file.
    File(PathBuf),
}

impl fmt::Display for SecretSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretSource::Config => write!(f, "config"),
            SecretSource::Environment(var) => write!(f, "env:{var}"),
            SecretSource::File(path) => write!(f, "file:{}", path.display()),
        }
    }
}

/// Errors from secret store operations.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("secret not found: {0}")]
    NotFound(String),

    #[error("failed to read secret file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("secret '{name}' already exists")]
    Duplicate { name: String },

    #[error("environment variable '{0}' not set")]
    EnvNotSet(String),

    #[error("secret value is empty for '{0}'")]
    EmptyValue(String),

    #[error("failed to write secret file: {0}")]
    FileWrite(std::io::Error),
}

/// In-memory secret store with automatic zeroization.
///
/// All values are cleared from memory when the store is dropped.
/// The store tracks the source of each secret for audit purposes.
pub struct SecretStore {
    secrets: HashMap<String, SecretEntry>,
    sources: HashMap<String, SecretSource>,
}

impl SecretStore {
    /// Create an empty secret store.
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// Add a secret to the store.
    pub fn insert(&mut self, entry: SecretEntry, source: SecretSource) -> Result<(), SecretError> {
        if entry.value.is_empty() {
            return Err(SecretError::EmptyValue(entry.name.clone()));
        }
        let name = entry.name.clone();
        self.secrets.insert(name.clone(), entry);
        self.sources.insert(name, source);
        Ok(())
    }

    /// Retrieve a secret by name.
    pub fn get(&self, name: &str) -> Option<&SecretEntry> {
        self.secrets.get(name)
    }

    /// Check if a secret exists.
    pub fn contains(&self, name: &str) -> bool {
        self.secrets.contains_key(name)
    }

    /// Get the source of a secret.
    pub fn source(&self, name: &str) -> Option<&SecretSource> {
        self.sources.get(name)
    }

    /// List all secret names (without exposing values).
    pub fn names(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }

    /// Number of secrets in the store.
    pub fn len(&self) -> usize {
        self.secrets.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }

    /// Remove a secret by name.
    pub fn remove(&mut self, name: &str) -> Option<SecretEntry> {
        self.sources.remove(name);
        self.secrets.remove(name)
    }

    /// Load a secret from an environment variable.
    ///
    /// Convention: looks for `CRUSTYCLAW_SECRET_<NAME>` (uppercased).
    pub fn load_from_env(
        &mut self,
        name: &str,
        injection: InjectionMethod,
    ) -> Result<(), SecretError> {
        let env_key = format!("CRUSTYCLAW_SECRET_{}", name.to_uppercase());
        let value = std::env::var(&env_key).map_err(|_| SecretError::EnvNotSet(env_key.clone()))?;

        let entry = SecretEntry {
            name: name.to_string(),
            value: SecretValue::new(value),
            injection,
            description: format!("Loaded from environment variable {env_key}"),
        };

        self.insert(entry, SecretSource::Environment(env_key))
    }

    /// Load a secret from a file.
    ///
    /// The file contents (trimmed of trailing newlines) become the secret value.
    pub fn load_from_file(
        &mut self,
        name: &str,
        path: &Path,
        injection: InjectionMethod,
    ) -> Result<(), SecretError> {
        let content = std::fs::read_to_string(path).map_err(|e| SecretError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        let value = content.trim_end_matches('\n').to_string();
        let entry = SecretEntry {
            name: name.to_string(),
            value: SecretValue::new(value),
            injection,
            description: format!("Loaded from file {}", path.display()),
        };

        self.insert(entry, SecretSource::File(path.to_path_buf()))
    }

    /// Generate environment variables for container injection.
    ///
    /// Returns a map of env var name → secret value for all secrets
    /// configured with [`InjectionMethod::Env`] or [`InjectionMethod::Both`].
    pub fn env_injections(&self) -> HashMap<String, String> {
        let mut envs = HashMap::new();
        for entry in self.secrets.values() {
            match &entry.injection {
                InjectionMethod::Env(var_name) => {
                    envs.insert(var_name.clone(), entry.value.expose().to_string());
                }
                InjectionMethod::Both { env_name, .. } => {
                    envs.insert(env_name.clone(), entry.value.expose().to_string());
                }
                InjectionMethod::File(_) => {}
            }
        }
        envs
    }

    /// Generate file injection specifications.
    ///
    /// Returns a list of `(guest_path, secret_value)` tuples for all secrets
    /// configured with [`InjectionMethod::File`] or [`InjectionMethod::Both`].
    pub fn file_injections(&self) -> Vec<FileInjection> {
        let mut files = Vec::new();
        for entry in self.secrets.values() {
            match &entry.injection {
                InjectionMethod::File(path) => {
                    files.push(FileInjection {
                        guest_path: path.clone(),
                        content: entry.value.expose().to_string(),
                        secret_name: entry.name.clone(),
                    });
                }
                InjectionMethod::Both { file_path, .. } => {
                    files.push(FileInjection {
                        guest_path: file_path.clone(),
                        content: entry.value.expose().to_string(),
                        secret_name: entry.name.clone(),
                    });
                }
                InjectionMethod::Env(_) => {}
            }
        }
        files
    }

    /// Write file-injected secrets to a staging directory on the host.
    ///
    /// This prepares secrets for bind-mounting into containers. Each secret
    /// gets its own file under `staging_dir` with restrictive permissions.
    ///
    /// Returns the list of created files (host_path, guest_path) for mounting.
    pub fn stage_file_injections(
        &self,
        staging_dir: &Path,
    ) -> Result<Vec<StagedSecret>, SecretError> {
        let mut staged = Vec::new();

        for injection in self.file_injections() {
            // Create a host-side file named after the secret
            let host_file = staging_dir.join(&injection.secret_name);
            std::fs::write(&host_file, injection.content.as_bytes())
                .map_err(SecretError::FileWrite)?;

            // Set restrictive permissions (owner read-only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o400);
                std::fs::set_permissions(&host_file, perms).map_err(SecretError::FileWrite)?;
            }

            staged.push(StagedSecret {
                host_path: host_file,
                guest_path: injection.guest_path.clone(),
                secret_name: injection.secret_name.clone(),
            });
        }

        Ok(staged)
    }
}

impl Default for SecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SecretStore {
    fn drop(&mut self) {
        // SecretEntry values are individually zeroized on drop via SecretValue,
        // but we also clear the maps to ensure no references linger.
        self.secrets.clear();
        self.sources.clear();
    }
}

impl fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretStore")
            .field("count", &self.secrets.len())
            .field("names", &self.names())
            .finish()
    }
}

/// A file injection specification (secret name, content, guest path).
pub struct FileInjection {
    /// Path inside the container where the secret file will appear.
    pub guest_path: PathBuf,
    /// The secret content to write. Zeroized when this struct is dropped.
    pub content: String,
    /// Name of the secret (for logging/audit).
    pub secret_name: String,
}

impl Drop for FileInjection {
    fn drop(&mut self) {
        self.content.zeroize();
    }
}

/// A secret file staged on the host, ready for bind-mounting.
#[derive(Debug, Clone)]
pub struct StagedSecret {
    /// Path on the host where the secret file was written.
    pub host_path: PathBuf,
    /// Path inside the container where the secret should appear.
    pub guest_path: PathBuf,
    /// Name of the secret (for logging/audit).
    pub secret_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_value_redacted_debug() {
        let secret = SecretValue::new("super-secret-api-key");
        let debug = format!("{secret:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("super-secret-api-key"));
    }

    #[test]
    fn test_secret_value_expose() {
        let secret = SecretValue::new("my-key-123");
        assert_eq!(secret.expose(), "my-key-123");
        assert_eq!(secret.len(), 10);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secret_store_basic() {
        let mut store = SecretStore::new();
        assert!(store.is_empty());

        let entry = SecretEntry {
            name: "api_key".to_string(),
            value: SecretValue::new("sk-1234567890"),
            injection: InjectionMethod::Env("API_KEY".to_string()),
            description: "Test API key".to_string(),
        };

        store.insert(entry, SecretSource::Config).unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.contains("api_key"));
        assert_eq!(store.source("api_key"), Some(&SecretSource::Config));

        let retrieved = store.get("api_key").unwrap();
        assert_eq!(retrieved.value.expose(), "sk-1234567890");
    }

    #[test]
    fn test_secret_store_rejects_empty() {
        let mut store = SecretStore::new();
        let entry = SecretEntry {
            name: "empty".to_string(),
            value: SecretValue::new(""),
            injection: InjectionMethod::Env("EMPTY".to_string()),
            description: String::new(),
        };

        let result = store.insert(entry, SecretSource::Config);
        assert!(matches!(result, Err(SecretError::EmptyValue(_))));
    }

    #[test]
    fn test_secret_store_remove() {
        let mut store = SecretStore::new();
        let entry = SecretEntry {
            name: "temp".to_string(),
            value: SecretValue::new("temporary"),
            injection: InjectionMethod::Env("TEMP".to_string()),
            description: String::new(),
        };
        store.insert(entry, SecretSource::Config).unwrap();
        assert!(store.contains("temp"));

        store.remove("temp");
        assert!(!store.contains("temp"));
    }

    #[test]
    fn test_env_injections() {
        let mut store = SecretStore::new();

        store
            .insert(
                SecretEntry {
                    name: "key_a".to_string(),
                    value: SecretValue::new("value_a"),
                    injection: InjectionMethod::Env("KEY_A".to_string()),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        store
            .insert(
                SecretEntry {
                    name: "key_b".to_string(),
                    value: SecretValue::new("value_b"),
                    injection: InjectionMethod::File(PathBuf::from("/run/secrets/key_b")),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        store
            .insert(
                SecretEntry {
                    name: "key_c".to_string(),
                    value: SecretValue::new("value_c"),
                    injection: InjectionMethod::Both {
                        env_name: "KEY_C".to_string(),
                        file_path: PathBuf::from("/run/secrets/key_c"),
                    },
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        let envs = store.env_injections();
        assert_eq!(envs.len(), 2); // key_a (Env) + key_c (Both)
        assert_eq!(envs.get("KEY_A").unwrap(), "value_a");
        assert_eq!(envs.get("KEY_C").unwrap(), "value_c");
        assert!(!envs.contains_key("key_b"));
    }

    #[test]
    fn test_file_injections() {
        let mut store = SecretStore::new();

        store
            .insert(
                SecretEntry {
                    name: "cert".to_string(),
                    value: SecretValue::new("-----BEGIN CERT-----"),
                    injection: InjectionMethod::File(PathBuf::from("/run/secrets/cert.pem")),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        store
            .insert(
                SecretEntry {
                    name: "token".to_string(),
                    value: SecretValue::new("tok-abc"),
                    injection: InjectionMethod::Env("TOKEN".to_string()),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        let files = store.file_injections();
        assert_eq!(files.len(), 1); // only cert (File)
        assert_eq!(files[0].guest_path, PathBuf::from("/run/secrets/cert.pem"));
        assert_eq!(files[0].content, "-----BEGIN CERT-----");
    }

    #[test]
    fn test_stage_file_injections() {
        let mut store = SecretStore::new();
        store
            .insert(
                SecretEntry {
                    name: "db_password".to_string(),
                    value: SecretValue::new("s3cret!"),
                    injection: InjectionMethod::File(PathBuf::from("/run/secrets/db_password")),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        let staging_dir = std::env::temp_dir().join("crustyclaw-test-staging");
        std::fs::create_dir_all(&staging_dir).unwrap();

        let staged = store.stage_file_injections(&staging_dir).unwrap();
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].secret_name, "db_password");
        assert_eq!(
            staged[0].guest_path,
            PathBuf::from("/run/secrets/db_password")
        );

        // Verify the file was written
        let content = std::fs::read_to_string(&staged[0].host_path).unwrap();
        assert_eq!(content, "s3cret!");

        // Cleanup
        std::fs::remove_dir_all(&staging_dir).ok();
    }

    #[test]
    fn test_secret_store_debug_redacted() {
        let mut store = SecretStore::new();
        store
            .insert(
                SecretEntry {
                    name: "api_key".to_string(),
                    value: SecretValue::new("super-secret"),
                    injection: InjectionMethod::Env("API_KEY".to_string()),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        let debug = format!("{store:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("api_key"));
    }

    #[test]
    fn test_secret_source_display() {
        assert_eq!(SecretSource::Config.to_string(), "config");
        assert_eq!(
            SecretSource::Environment("MY_VAR".to_string()).to_string(),
            "env:MY_VAR"
        );
        assert_eq!(
            SecretSource::File(PathBuf::from("/etc/secret")).to_string(),
            "file:/etc/secret"
        );
    }

    #[test]
    fn test_load_from_env() {
        // Set an env var for testing
        std::env::set_var("CRUSTYCLAW_SECRET_TEST_KEY", "env-secret-value");

        let mut store = SecretStore::new();
        store
            .load_from_env("test_key", InjectionMethod::Env("TEST_KEY".to_string()))
            .unwrap();

        let entry = store.get("test_key").unwrap();
        assert_eq!(entry.value.expose(), "env-secret-value");
        assert_eq!(
            store.source("test_key"),
            Some(&SecretSource::Environment(
                "CRUSTYCLAW_SECRET_TEST_KEY".to_string()
            ))
        );

        // Cleanup
        std::env::remove_var("CRUSTYCLAW_SECRET_TEST_KEY");
    }

    #[test]
    fn test_load_from_env_not_set() {
        let mut store = SecretStore::new();
        let result = store.load_from_env(
            "nonexistent",
            InjectionMethod::Env("NONEXISTENT".to_string()),
        );
        assert!(matches!(result, Err(SecretError::EnvNotSet(_))));
    }
}
