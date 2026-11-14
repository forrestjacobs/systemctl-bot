use crate::config::{Command, SystemctlBotConfig};
use crate::systemctl::{Systemctl, SystemctlError};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub enum UserCommand {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
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
        }
    }
}
