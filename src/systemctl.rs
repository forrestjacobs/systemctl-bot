use anyhow::{bail, Result};
use async_trait::async_trait;
use mockall::mock;
use std::{any::Any, process::Output};
use tokio::process::Command;

#[async_trait]
pub trait Systemctl: Any + Sync + Send {
    async fn run(&self, args: &[&str]) -> Result<()>;
}

mock! {
    pub Systemctl {
        pub async fn run(&self, args: Vec<String>) -> Result<()>;
    }
}

#[async_trait]
impl Systemctl for MockSystemctl {
    async fn run(&self, args: &[&str]) -> Result<()> {
        self.run(args.into_iter().map(|s| s.to_string()).collect())
            .await
    }
}

pub struct SystemctlImpl;

impl SystemctlImpl {
    fn respond(output: Output) -> Result<()> {
        if output.status.success() {
            Ok(())
        } else {
            bail!(
                "process failed with {}\n\n{}",
                output.status,
                String::from_utf8(output.stderr).unwrap()
            )
        }
    }
}

#[async_trait]
impl Systemctl for SystemctlImpl {
    async fn run(&self, args: &[&str]) -> Result<()> {
        let mut command = Command::new("systemctl");
        let output = command.args(args).output().await?;
        Self::respond(output)
    }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::process::ExitStatusExt, process::ExitStatus};

    use super::*;

    #[test]
    fn test_success_response() {
        assert_eq!(
            SystemctlImpl::respond(Output {
                status: ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
            .ok(),
            Some(())
        );
    }

    #[test]
    fn test_error_response() {
        assert_eq!(
            SystemctlImpl::respond(Output {
                status: ExitStatus::from_raw(9),
                stdout: Vec::new(),
                stderr: "Example out".as_bytes().into(),
            })
            .map_err(|e| e.to_string()),
            Err("process failed with signal: 9 (SIGKILL)\n\nExample out".to_string())
        );
    }
}
