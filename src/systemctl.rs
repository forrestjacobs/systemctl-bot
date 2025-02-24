use async_trait::async_trait;
use shaku::{Component, Interface};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{ExitStatus, Output};
use tokio::process::Command;

#[derive(Debug)]
pub enum SystemctlError {
    IoError(io::Error),
    NonZeroExit { status: ExitStatus, stderr: String },
}

impl Display for SystemctlError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SystemctlError::IoError(e) => write!(f, "{}", e),
            SystemctlError::NonZeroExit { status, stderr } => {
                write!(f, "systemctl failed with {}\n\n{}", status, stderr)
            }
        }
    }
}

impl Error for SystemctlError {}

impl From<io::Error> for SystemctlError {
    fn from(error: io::Error) -> Self {
        SystemctlError::IoError(error)
    }
}

fn to_str(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

impl From<Output> for SystemctlError {
    fn from(output: Output) -> Self {
        SystemctlError::NonZeroExit {
            status: output.status,
            stderr: to_str(output.stderr),
        }
    }
}

#[async_trait]
pub trait Systemctl: Interface {
    async fn start(&self, unit: &str) -> Result<(), SystemctlError>;
    async fn stop(&self, unit: &str) -> Result<(), SystemctlError>;
    async fn restart(&self, unit: &str) -> Result<(), SystemctlError>;
}

#[derive(Component)]
#[shaku(interface = Systemctl)]
pub struct SystemctlImpl;

impl SystemctlImpl {
    async fn run<S: AsRef<OsStr>, T: AsRef<OsStr>>(verb: S, unit: T) -> Result<(), SystemctlError> {
        let output = Command::new("systemctl")
            .arg(verb)
            .arg(unit)
            .output()
            .await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(SystemctlError::from(output))
        }
    }
}

#[async_trait]
impl Systemctl for SystemctlImpl {
    async fn start(&self, unit: &str) -> Result<(), SystemctlError> {
        Self::run("start", &unit).await
    }
    async fn stop(&self, unit: &str) -> Result<(), SystemctlError> {
        Self::run("stop", &unit).await
    }
    async fn restart(&self, unit: &str) -> Result<(), SystemctlError> {
        Self::run("restart", &unit).await
    }
}
