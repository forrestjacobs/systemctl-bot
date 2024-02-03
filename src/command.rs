use crate::systemctl::{restart, start, stop, SystemctlError};
use crate::systemd_status::SystemdStatusManager;
use crate::units::{UnitPermissions, Units, UnitsTrait};
use itertools::Itertools;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
    SingleStatus { unit: String },
    MultiStatus,
}

#[derive(Debug)]
pub enum UserCommandError {
    SystemctlError(SystemctlError),
    ZbusError(zbus::Error),
    NotAllowed,
}

impl Display for UserCommandError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            UserCommandError::SystemctlError(e) => write!(f, "{}", e),
            UserCommandError::ZbusError(e) => write!(f, "{}", e),
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

impl From<zbus::Error> for UserCommandError {
    fn from(error: zbus::Error) -> Self {
        UserCommandError::ZbusError(error)
    }
}

fn ensure_allowed(
    units: &Units,
    unit: &str,
    permissions: UnitPermissions,
) -> Result<(), UserCommandError> {
    if units
        .get(unit)
        .map_or(false, |unit| unit.permissions.contains(permissions))
    {
        Ok(())
    } else {
        Err(UserCommandError::NotAllowed)
    }
}

impl UserCommand {
    pub async fn run(
        &self,
        units: &Units,
        systemd_status_manager: &SystemdStatusManager,
    ) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Start)?;
                start(unit).await?;
                Ok(format!("Started {}", unit))
            }
            UserCommand::Stop { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Stop)?;
                stop(unit).await?;
                Ok(format!("Stopped {}", unit))
            }
            UserCommand::Restart { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Stop | UnitPermissions::Start)?;
                restart(unit).await?;
                Ok(format!("Restarted {}", unit))
            }
            UserCommand::SingleStatus { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Status)?;
                Ok(systemd_status_manager.status(unit).await?)
            }
            UserCommand::MultiStatus => {
                let mut status_lines = systemd_status_manager
                    .statuses(units.with_permissions(UnitPermissions::Status))
                    .await
                    .map(|(unit, status)| (unit, status.unwrap_or_else(|err| format!("{}", err))))
                    .filter(|(_, status)| status != "inactive")
                    .map(|(unit, status)| format!("{}: {}", unit, status))
                    .peekable();
                let response = if status_lines.peek().is_none() {
                    String::from("Nothing is active")
                } else {
                    status_lines.join("\n")
                };
                Ok(response)
            }
        }
    }
}
