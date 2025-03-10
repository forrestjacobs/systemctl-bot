use crate::{
    commands::get_commands,
    config::{CommandType, UnitCollection},
    status_monitor::StatusMonitor,
    systemctl::Systemctl,
    systemd_status::SystemdStatusManager,
};
use anyhow::Error;
use anyhow::Result;
use mockall::automock;
use poise::serenity_prelude::{ApplicationId, Client, GatewayIntents};
use poise::{samples::register_in_guild, serenity_prelude::GuildId, Framework, FrameworkOptions};
use std::sync::Arc;

pub struct Data {
    pub units: Arc<UnitCollection>,
    pub systemctl: Arc<dyn Systemctl>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

#[automock]
pub trait CommandContext {
    async fn defer_response(&self) -> Result<()>;
    async fn respond(&self, response: String) -> Result<()>;

    fn get_command_name(&self) -> &str;
    fn get_units(&self) -> &Arc<UnitCollection>;
    fn get_systemctl(&self) -> Arc<dyn Systemctl>;
    fn get_systemd_status_manager(&self) -> Arc<dyn SystemdStatusManager>;
}

impl CommandContext for Context<'_> {
    async fn defer_response(&self) -> Result<()> {
        self.defer().await?;
        Ok(())
    }

    async fn respond(&self, response: String) -> Result<()> {
        self.say(response).await?;
        Ok(())
    }

    fn get_command_name(&self) -> &str {
        &self.command().name
    }

    fn get_units(&self) -> &Arc<UnitCollection> {
        &self.data().units
    }
    fn get_systemctl(&self) -> Arc<dyn Systemctl> {
        self.data().systemctl.clone()
    }
    fn get_systemd_status_manager(&self) -> Arc<dyn SystemdStatusManager> {
        self.data().systemd_status_manager.clone()
    }
}

pub fn build_framework(
    guild_id: GuildId,
    command_type: CommandType,
    status_monitor: Arc<dyn StatusMonitor>,
    data: Arc<Data>,
) -> Framework<Arc<Data>, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: get_commands(command_type, &data.units),
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

pub async fn start_client(
    discord_token: String,
    application_id: ApplicationId,
    framework: Framework<Arc<Data>, Error>,
) -> Result<()> {
    Client::builder(
        discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .framework(framework)
    .application_id(application_id)
    .await?
    .start()
    .await?;
    Ok(())
}
