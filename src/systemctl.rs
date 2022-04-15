use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{Command, ExitStatus};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum SystemctlError {
    IoError(io::Error),
    NonZeroExit {
        status: ExitStatus,
        stderr: Result<String, FromUtf8Error>,
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
                    .as_ref()
                    .unwrap_or(&String::from("unable to parse stdout"))
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

fn run(command: &mut Command) -> Result<Vec<u8>, SystemctlError> {
    let output = command.output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(SystemctlError::NonZeroExit {
            status: output.status,
            stderr: String::from_utf8(output.stderr),
        })
    }
}

pub fn start<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    run(Command::new("systemctl").arg("start").arg(&unit))?;
    Ok(())
}

pub fn stop<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    run(Command::new("systemctl").arg("stop").arg(&unit))?;
    Ok(())
}
