use crate::config::{Command, Config};
use crate::systemctl::{Systemctl, SystemctlError};
use crate::systemd_status::SystemdStatusManager;
use poise::command;
use poise::serenity_prelude::AutocompleteChoice;
use shaku::{Component, Interface};
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::iter::empty;
use std::sync::Arc;

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

pub trait Data: Interface {
    fn config(&self) -> &dyn Config;
    fn systemctl(&self) -> &dyn Systemctl;
    fn systemd_status_manager(&self) -> &dyn SystemdStatusManager;
    fn ensure_allowed(&self, unit: &String, command: Command) -> Result<(), CommandRunnerError>;
}

#[derive(Component)]
#[shaku(interface = Data)]
pub struct DataImpl {
    pub config: Arc<dyn Config>,
    pub systemctl: Arc<dyn Systemctl>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

impl Data for DataImpl {
    fn config(&self) -> &dyn Config {
        self.config.as_ref()
    }
    fn systemctl(&self) -> &dyn Systemctl {
        self.systemctl.as_ref()
    }
    fn systemd_status_manager(&self) -> &dyn SystemdStatusManager {
        self.systemd_status_manager.as_ref()
    }
    fn ensure_allowed(&self, unit: &String, command: Command) -> Result<(), CommandRunnerError> {
        if self.config.units[&command].contains(unit) {
            Ok(())
        } else {
            Err(CommandRunnerError::NotAllowed)
        }
    }
}

type CommandError = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Arc<dyn Data>, CommandError>;

async fn autocomplete_units<'a>(ctx: Context<'a>, partial: &'a str) -> Vec<AutocompleteChoice> {
    let Ok(command) = Command::try_from(ctx.command().name.as_str()) else {
        return empty().collect();
    };
    ctx.data().config().units[&command]
        .iter()
        .filter(move |unit| unit.starts_with(partial))
        .map(|unit| {
            let alias = unit.strip_suffix(".service").unwrap_or(unit);
            AutocompleteChoice::new(alias, unit.as_str())
        })
        .collect()
}

/// Starts units
#[command(slash_command)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "The unit to start"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Start)?;
    data.systemctl().start(&unit).await?;
    ctx.say(format!("Started {}", unit)).await?;
    Ok(())
}

/// Stops units
#[command(slash_command)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "The unit to stop"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Stop)?;
    data.systemctl().stop(&unit).await?;
    ctx.say(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Restarts units
#[command(slash_command)]
pub async fn restart(
    ctx: Context<'_>,
    #[description = "The unit to restart"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Restart)?;
    data.systemctl().restart(&unit).await?;
    ctx.say(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Checks units' status
#[command(slash_command)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "The unit to check"]
    #[autocomplete = "autocomplete_units"]
    unit: Option<String>,
) -> Result<(), CommandError> {
    ctx.defer().await?;
    let data = ctx.data();
    let response = match unit {
        Some(unit) => {
            data.ensure_allowed(&unit, Command::Status)?;
            data.systemd_status_manager().status(&unit).await?
        }
        None => {
            let lines = data
                .systemd_status_manager()
                .statuses(&data.config().units[&Command::Status])
                .await
                .map(|(unit, status)| (unit, status.unwrap_or_else(|err| format!("{}", err))))
                .filter(|(_, status)| status != "inactive")
                .map(|(unit, status)| format!("{}: {}", unit, status))
                .collect::<Vec<String>>();
            if lines.is_empty() {
                String::from("Nothing is active")
            } else {
                lines.join("\n")
            }
        }
    };
    ctx.say(response).await?;
    Ok(())
}
