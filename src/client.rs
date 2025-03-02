use crate::{
    commands::get_commands,
    config::{CommandType, UnitCollection},
    process::ProcessRunner,
    status_monitor::StatusMonitor,
    systemd_status::SystemdStatusManager,
};
use poise::{samples::register_in_guild, serenity_prelude::GuildId, Framework, FrameworkOptions};
use std::sync::Arc;

pub struct Data {
    pub units: Arc<UnitCollection>,
    pub runner: Arc<dyn ProcessRunner>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
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
