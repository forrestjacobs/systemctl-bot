use async_trait::async_trait;
use std::any::Any;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{ExitStatus, Output};
use tokio::process::Command;

#[derive(Debug)]
pub enum ProcessError {
    IoError(io::Error),
    NonZeroExit { status: ExitStatus, stderr: String },
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ProcessError::IoError(e) => write!(f, "{}", e),
            ProcessError::NonZeroExit { status, stderr } => {
                write!(f, "process failed with {}\n\n{}", status, stderr)
            }
        }
    }
}

impl Error for ProcessError {}

impl From<io::Error> for ProcessError {
    fn from(error: io::Error) -> Self {
        ProcessError::IoError(error)
    }
}

fn to_str(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

impl From<Output> for ProcessError {
    fn from(output: Output) -> Self {
        ProcessError::NonZeroExit {
            status: output.status,
            stderr: to_str(output.stderr),
        }
    }
}

#[async_trait]
pub trait ProcessRunner: Any + Sync + Send {
    async fn run(&self, command: &mut Command) -> Result<(), ProcessError>;
}

pub struct ProcessRunnerImpl;

#[async_trait]
impl ProcessRunner for ProcessRunnerImpl {
    async fn run(&self, command: &mut Command) -> Result<(), ProcessError> {
        let output = command.output().await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(ProcessError::from(output))
        }
    }
}
