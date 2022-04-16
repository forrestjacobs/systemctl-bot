use std::process::Output;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub enum SystemctlError {
    IoError(io::Error),
    NonZeroExit {
        status: ExitStatus,
        stderr: String,
    },
}

impl Display for SystemctlError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SystemctlError::IoError(e) => write!(f, "{}", e),
            SystemctlError::NonZeroExit { status, stderr } => write!(
                f,
                "systemctl exited with status {}: {}",
                status,
                stderr
            ),
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

fn to_nonzero_error_result(output: Output) -> Result<(), SystemctlError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(SystemctlError::from(output))
    }
}

pub fn start<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    to_nonzero_error_result(Command::new("systemctl").arg("start").arg(&unit).output()?)
}

pub fn stop<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    to_nonzero_error_result(Command::new("systemctl").arg("stop").arg(&unit).output()?)
}
