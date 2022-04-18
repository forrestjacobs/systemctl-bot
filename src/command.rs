use crate::config::{Unit, UnitPermission};
use crate::systemctl::{restart, start, status, stop, SystemctlError};
use futures::future::join_all;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand<'a> {
    Start { unit: &'a Unit },
    Stop { unit: &'a Unit },
    Restart { unit: &'a Unit },
    Status { units: Vec<&'a Unit> },
}

#[derive(Debug)]
pub enum UserCommandError {
    SystemctlError(SystemctlError),
    NotAllowed,
}

impl Display for UserCommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            UserCommandError::SystemctlError(e) => write!(f, "{}", e),
            UserCommandError::NotAllowed => {
                write!(f, "Command is not allowed")
            }
        }
    }
}

impl Error for UserCommandError {}

impl From<SystemctlError> for UserCommandError {
    fn from(error: SystemctlError) -> Self {
        UserCommandError::SystemctlError(error)
    }
}

fn ensure_allowed(unit: &Unit, permission: UnitPermission) -> Result<(), UserCommandError> {
    if unit.permissions.contains(&permission) {
        Ok(())
    } else {
        Err(UserCommandError::NotAllowed)
    }
}

impl UserCommand<'_> {
    pub async fn run(&self) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { unit } => {
                ensure_allowed(unit, UnitPermission::Start)?;
                start(&unit.name).await?;
                Ok(format!("Started {}", unit.name))
            }
            UserCommand::Stop { unit } => {
                ensure_allowed(unit, UnitPermission::Stop)?;
                stop(&unit.name).await?;
                Ok(format!("Stopped {}", unit.name))
            }
            UserCommand::Restart { unit } => {
                ensure_allowed(unit, UnitPermission::Stop)?;
                ensure_allowed(unit, UnitPermission::Start)?;
                restart(&unit.name).await?;
                Ok(format!("Restarted {}", unit.name))
            }
            UserCommand::Status { units } => {
                for unit in units {
                    ensure_allowed(unit, UnitPermission::Status)?;
                }
                let statuses = join_all(units.iter().map(|unit| status(&unit.name))).await;
                let response = units
                    .iter()
                    .zip(statuses)
                    .map(|(unit, status)| -> Result<String, SystemctlError> {
                        status.map(|status| format!("{} status: {}", &unit.name, status))
                    })
                    .collect::<Result<Vec<String>, SystemctlError>>()?
                    .join("\n");
                Ok(response)
            }
        }
    }
}
