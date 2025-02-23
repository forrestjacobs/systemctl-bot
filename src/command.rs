use crate::config::{Command, Config};
use crate::systemctl::{restart, start, stop, SystemctlError};
use crate::systemd_status::{statuses, SystemdStatusManager};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

pub enum UserCommand {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
    SingleStatus { unit: String },
    MultiStatus { units: Vec<String> },
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

fn ensure_allowed<C: Config + ?Sized>(
    unit: &String,
    command: Command,
    config: &C,
) -> Result<(), UserCommandError> {
    if config.get().units[&command].contains(unit) {
        Ok(())
    } else {
        Err(UserCommandError::NotAllowed)
    }
}

impl UserCommand {
    pub async fn run<M: SystemdStatusManager + ?Sized, C: Config + ?Sized>(
        &self,
        systemd_status_manager: &M,
        config: &C,
    ) -> Result<String, UserCommandError> {
        match self {
            UserCommand::Start { unit } => {
                ensure_allowed(unit, Command::Start, config)?;
                start(unit).await?;
                Ok(format!("Started {}", unit))
            }
            UserCommand::Stop { unit } => {
                ensure_allowed(unit, Command::Stop, config)?;
                stop(unit).await?;
                Ok(format!("Stopped {}", unit))
            }
            UserCommand::Restart { unit } => {
                ensure_allowed(unit, Command::Restart, config)?;
                restart(unit).await?;
                Ok(format!("Restarted {}", unit))
            }
            UserCommand::SingleStatus { unit } => {
                ensure_allowed(unit, Command::Status, config)?;
                Ok(systemd_status_manager.status(unit.as_str()).await?)
            }
            UserCommand::MultiStatus { units } => {
                for unit in units {
                    ensure_allowed(unit, Command::Status, config)?;
                }

                let statuses =
                    statuses(systemd_status_manager, units.iter().map(|u| u.as_str())).await;
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
