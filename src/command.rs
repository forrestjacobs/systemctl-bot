use crate::config::{Unit, UnitPermission};
use crate::systemctl::{statuses, SystemctlError, SystemctlManager};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand<'a> {
    Start { unit: &'a Unit },
    Stop { unit: &'a Unit },
    Restart { unit: &'a Unit },
    SingleStatus { unit: &'a Unit },
    MultiStatus { units: Vec<&'a Unit> },
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
    pub async fn run<'a>(
        &self,
        systemctl: &SystemctlManager<'a>,
    ) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { unit } => {
                ensure_allowed(unit, UnitPermission::Start)?;
                systemctl.start(&unit.name).await?;
                Ok(format!("Started {}", unit.name))
            }
            UserCommand::Stop { unit } => {
                ensure_allowed(unit, UnitPermission::Stop)?;
                systemctl.stop(&unit.name).await?;
                Ok(format!("Stopped {}", unit.name))
            }
            UserCommand::Restart { unit } => {
                ensure_allowed(unit, UnitPermission::Stop)?;
                ensure_allowed(unit, UnitPermission::Start)?;
                systemctl.restart(&unit.name).await?;
                Ok(format!("Restarted {}", unit.name))
            }
            UserCommand::SingleStatus { unit } => {
                ensure_allowed(unit, UnitPermission::Status)?;
                Ok(systemctl.status(unit.name.as_str()).await?)
            }
            UserCommand::MultiStatus { units } => {
                for unit in units {
                    ensure_allowed(unit, UnitPermission::Status)?;
                }

                let statuses = statuses(systemctl, units.iter().map(|u| u.name.as_str())).await;
                let status_lines = statuses
                    .into_iter()
                    .map(|(unit, status)| (unit, status.unwrap_or_else(|err| format!("{}", err))))
                    .filter(|(_, status)| status != "inactive")
                    .map(|(unit, status)| format!("{}: {}", unit, status))
                    .collect::<Vec<String>>();
                let response = if status_lines.is_empty() {
                    String::from("Nothing is active")
                } else {
                    status_lines.join("\n")
                };
                Ok(response)
            }
        }
    }
}
