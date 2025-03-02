use anyhow::{bail, Result};
use async_trait::async_trait;
use mockall::mock;
use std::any::Any;
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

#[async_trait]
impl Systemctl for SystemctlImpl {
    async fn run(&self, args: &[&str]) -> Result<()> {
        let mut command = Command::new("systemctl");
        let output = command.args(args).output().await?;
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
