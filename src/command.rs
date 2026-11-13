use crate::config::{Command, SystemctlBotConfig};
use crate::systemctl::{Systemctl, SystemctlError};
use crate::systemd_status::SystemdStatusManager;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub enum UserCommand {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
    SingleStatus { unit: String },
    MultiStatus { units: Vec<String> },
}

#[derive(Debug)]
pub enum CommandRunnerError {
    SystemctlError(SystemctlError),
    ZbusError(zbus::Error),
    NotAllowed,
}

impl Display for CommandRunnerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandRunnerError::SystemctlError(e) => write!(f, "{}", e),
            CommandRunnerError::ZbusError(e) => write!(f, "{}", e),
            CommandRunnerError::NotAllowed => {
                write!(f, "Command is not allowed")
            }
        }
    }
}

impl Error for CommandRunnerError {}

impl From<SystemctlError> for CommandRunnerError {
    fn from(error: SystemctlError) -> Self {
        CommandRunnerError::SystemctlError(error)
    }
}

impl From<zbus::Error> for CommandRunnerError {
    fn from(error: zbus::Error) -> Self {
        CommandRunnerError::ZbusError(error)
    }
}

pub struct CommandRunner {
    pub config: Arc<SystemctlBotConfig>,
    pub systemctl: Arc<Systemctl>,
    pub systemd_status_manager: Arc<SystemdStatusManager>,
}

impl CommandRunner {
    fn ensure_allowed(&self, unit: &String, command: Command) -> Result<(), CommandRunnerError> {
        if self.config.units[&command].contains(unit) {
            Ok(())
        } else {
            Err(CommandRunnerError::NotAllowed)
        }
    }
}

impl CommandRunner {
    pub async fn run(&self, command: &UserCommand) -> Result<String, CommandRunnerError> {
        match command {
            UserCommand::Start { unit } => {
                self.ensure_allowed(unit, Command::Start)?;
                self.systemctl.start(unit).await?;
                Ok(format!("Started {}", unit))
            }
            UserCommand::Stop { unit } => {
                self.ensure_allowed(unit, Command::Stop)?;
                self.systemctl.stop(unit).await?;
                Ok(format!("Stopped {}", unit))
            }
            UserCommand::Restart { unit } => {
                self.ensure_allowed(unit, Command::Restart)?;
                self.systemctl.restart(unit).await?;
                Ok(format!("Restarted {}", unit))
            }
            UserCommand::SingleStatus { unit } => {
                self.ensure_allowed(unit, Command::Status)?;
                Ok(self.systemd_status_manager.status(unit.as_str()).await?)
            }
            UserCommand::MultiStatus { units } => {
                for unit in units {
                    self.ensure_allowed(unit, Command::Status)?;
                }

                let statuses = self
                    .systemd_status_manager
                    .statuses(units.iter().map(|u| u.as_str()))
                    .await;
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
