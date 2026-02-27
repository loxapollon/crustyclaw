//! No-op (development) sandbox backend.
//!
//! Runs commands directly on the host with no isolation. Resource limits
//! are logged but not enforced. **Never use in production.**

use crate::BoxFuture;

use super::{IsolationError, SandboxBackend, SandboxConfig, SandboxResult};

/// No-op sandbox backend for development and testing.
pub struct NoopBackend;

impl SandboxBackend for NoopBackend {
    fn name(&self) -> &str {
        "noop"
    }

    fn available(&self) -> bool {
        true
    }

    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> BoxFuture<'_, Result<SandboxResult, IsolationError>> {
        let label = config.label.clone();
        let timeout = config.limits.timeout;
        let workdir = config.workdir.clone();
        let env = config.env.clone();
        let cmd = command.to_vec();

        Box::pin(async move {
            tracing::warn!(
                backend = "noop",
                label = %label,
                "Running WITHOUT isolation (development mode)"
            );

            if cmd.is_empty() {
                return Err(IsolationError::Execution(
                    "command must not be empty".to_string(),
                ));
            }

            let start = std::time::Instant::now();

            let mut proc = tokio::process::Command::new(&cmd[0]);
            proc.args(&cmd[1..])
                .current_dir(&workdir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            for (k, v) in &env {
                proc.env(k, v);
            }

            let child = proc
                .spawn()
                .map_err(|e| IsolationError::Execution(format!("spawn failed: {e}")))?;

            let output = match timeout {
                Some(dur) => {
                    let result = tokio::time::timeout(dur, child.wait_with_output()).await;
                    match result {
                        Ok(Ok(output)) => output,
                        Ok(Err(e)) => {
                            return Err(IsolationError::Execution(format!("wait failed: {e}")));
                        }
                        Err(_) => {
                            return Err(IsolationError::Timeout(dur));
                        }
                    }
                }
                None => child
                    .wait_with_output()
                    .await
                    .map_err(|e| IsolationError::Execution(format!("wait failed: {e}")))?,
            };

            let elapsed = start.elapsed();

            Ok(SandboxResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                elapsed,
                peak_memory_bytes: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_noop_backend_available() {
        let backend = NoopBackend;
        assert!(backend.available());
        assert_eq!(backend.name(), "noop");
    }

    #[tokio::test]
    async fn test_noop_backend_execute_echo() {
        let config = SandboxConfig::new("echo-test")
            .with_timeout(Duration::from_secs(5))
            .with_workdir("/tmp");

        let backend = NoopBackend;
        let result = backend
            .execute(
                &config,
                &[
                    "echo".to_string(),
                    "hello".to_string(),
                    "sandbox".to_string(),
                ],
            )
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello sandbox");
    }

    #[tokio::test]
    async fn test_noop_backend_execute_failure() {
        let config = SandboxConfig::new("fail-test").with_workdir("/tmp");
        let backend = NoopBackend;
        let result = backend
            .execute(&config, &["false".to_string()])
            .await
            .unwrap();
        assert!(!result.success());
    }

    #[tokio::test]
    async fn test_noop_backend_timeout() {
        let config = SandboxConfig::new("timeout-test")
            .with_timeout(Duration::from_millis(100))
            .with_workdir("/tmp");

        let backend = NoopBackend;
        let result = backend
            .execute(&config, &["sleep".to_string(), "10".to_string()])
            .await;
        assert!(matches!(result, Err(IsolationError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_noop_backend_env_vars() {
        let config = SandboxConfig::new("env-test")
            .with_env("MY_VAR", "hello_from_sandbox")
            .with_workdir("/tmp");

        let backend = NoopBackend;
        let result = backend
            .execute(
                &config,
                &[
                    "sh".to_string(),
                    "-c".to_string(),
                    "echo $MY_VAR".to_string(),
                ],
            )
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello_from_sandbox");
    }
}
