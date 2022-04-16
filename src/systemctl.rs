use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{Command, ExitStatus, Output};

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
                write!(f, "systemctl exited with status {}: {}", status, stderr)
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

fn systemctl_do<S: AsRef<OsStr>, T: AsRef<OsStr>>(verb: S, unit: T) -> Result<(), SystemctlError> {
    let output = Command::new("systemctl").arg(verb).arg(unit).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SystemctlError::from(output))
    }
}

pub fn start<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("start", &unit)
}

pub fn stop<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("stop", &unit)
}

pub fn restart<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("restart", &unit)
}

pub fn status<S: AsRef<OsStr>>(unit: S) -> Result<String, SystemctlError> {
    let output = Command::new("systemctl")
        .arg("is-active")
        .arg(&unit)
        .output()?;
    Ok(to_str(output.stdout))
}
