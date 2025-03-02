use crate::{
    commands::get_commands,
    config::{CommandType, UnitCollection},
    status_monitor::StatusMonitor,
    systemctl::Systemctl,
    systemd_status::SystemdStatusManager,
};
use mockall::automock;
use poise::{
    samples::register_in_guild,
    serenity_prelude::{self, GuildId},
    Framework, FrameworkOptions,
};
use std::sync::Arc;

pub struct Data {
    pub units: Arc<UnitCollection>,
    pub systemctl: Arc<dyn Systemctl>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, anyhow::Error>;

#[automock]
pub trait CommandContext {
    async fn defer_response(&self) -> Result<(), serenity_prelude::Error>;
    fn get_data(&self) -> Arc<Data>;
    async fn respond(&self, response: String) -> Result<(), serenity_prelude::Error>;
}

impl CommandContext for Context<'_> {
    async fn defer_response(&self) -> Result<(), serenity_prelude::Error> {
        self.defer().await
    }

    fn get_data(&self) -> Arc<Data> {
        self.data().clone()
    }

    async fn respond(&self, response: String) -> Result<(), serenity_prelude::Error> {
        self.say(response).await?;
        Ok(())
    }
}

pub fn build_framework(
    guild_id: GuildId,
    command_type: CommandType,
    status_monitor: Arc<dyn StatusMonitor>,
    data: Arc<Data>,
) -> Framework<Arc<Data>, anyhow::Error> {
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
