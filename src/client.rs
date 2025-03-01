use crate::{
    commands::get_commands,
    config::{CommandType, UnitCollection},
    process::{ProcessError, ProcessRunner},
    status_monitor::StatusMonitor,
    systemd_status::SystemdStatusManager,
};
use poise::{samples::register_in_guild, serenity_prelude::GuildId, Framework, FrameworkOptions};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[derive(Debug)]
pub enum CommandRunnerError {
    ProcessError(ProcessError),
    ZbusError(zbus::Error),
    NotAllowed,
}

impl Display for CommandRunnerError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            CommandRunnerError::ProcessError(e) => write!(f, "{}", e),
            CommandRunnerError::ZbusError(e) => write!(f, "{}", e),
            CommandRunnerError::NotAllowed => {
                write!(f, "Command is not allowed")
            }
        }
    }
}

impl Error for CommandRunnerError {}

impl From<ProcessError> for CommandRunnerError {
    fn from(error: ProcessError) -> Self {
        CommandRunnerError::ProcessError(error)
    }
}

impl From<zbus::Error> for CommandRunnerError {
    fn from(error: zbus::Error) -> Self {
        CommandRunnerError::ZbusError(error)
    }
}

pub struct Data {
    pub units: Arc<UnitCollection>,
    pub runner: Arc<dyn ProcessRunner>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

impl Data {
    pub fn ensure_allowed(
        &self,
        unit: &String,
        command: crate::config::Command,
    ) -> Result<(), CommandRunnerError> {
        if self.units[&command].contains(unit) {
            Ok(())
        } else {
            Err(CommandRunnerError::NotAllowed)
        }
    }
}

pub type BoxedError = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Arc<Data>, BoxedError>;

pub fn build_framework(
    guild_id: GuildId,
    command_type: CommandType,
    status_monitor: Arc<dyn StatusMonitor>,
    data: Arc<Data>,
) -> Framework<Arc<Data>, BoxedError> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: get_commands(command_type),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                register_in_guild(&ctx.http, &framework.options().commands, guild_id).await?;
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    status_monitor.monitor(&ctx).await;
                });
                Ok(data)
            })
        })
        .build()
}
