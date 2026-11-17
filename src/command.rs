use crate::config::SystemctlBotConfig;
use crate::systemd::{Systemd, SystemdRunError};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    Restart,
}

impl Command {
    pub const VALUES: [Self; 3] = [Self::Start, Self::Stop, Self::Restart];
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start => write!(f, "start"),
            Self::Stop => write!(f, "stop"),
            Self::Restart => write!(f, "restart"),
        }
    }
}

#[derive(Debug)]
pub enum CommandRunnerError {
    SystemdError(SystemdRunError),
    NotAllowed,
}

impl Display for CommandRunnerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandRunnerError::SystemdError(e) => write!(f, "{}", e),
            CommandRunnerError::NotAllowed => {
                write!(f, "Command is not allowed")
            }
        }
    }
}

impl Error for CommandRunnerError {}

impl From<SystemdRunError> for CommandRunnerError {
    fn from(error: SystemdRunError) -> Self {
        CommandRunnerError::SystemdError(error)
    }
}

pub struct CommandRunner {
    pub config: Arc<SystemctlBotConfig>,
    pub systemd: Arc<Systemd>,
}

impl CommandRunner {
    fn ensure_allowed(&self, command: &Command, unit: &str) -> Result<(), CommandRunnerError> {
        if self.config.units.command_units[command]
            .iter()
            .any(|u| u == unit)
        {
            Ok(())
        } else {
            Err(CommandRunnerError::NotAllowed)
        }
    }
}

impl CommandRunner {
    pub async fn run(&self, command: &Command, unit: &str) -> Result<String, CommandRunnerError> {
        self.ensure_allowed(command, unit)?;
        self.systemd.run(command, unit).await?;
        let verb = match command {
            Command::Start => "Started",
            Command::Stop => "Stopped",
            Command::Restart => "Restarted",
        };
        Ok(format!("{} {}", verb, unit))
    }
}
