use crate::{
    commands::get_commands,
    config::{CommandType, UnitCollection, UnitSection},
    status_monitor::monitor_status,
    systemd::SystemdManager,
};
use anyhow::Error;
use anyhow::Result;
use mockall::automock;
use poise::serenity_prelude::{ApplicationId, Client, GatewayIntents};
use poise::{samples::register_in_guild, serenity_prelude::GuildId, Framework, FrameworkOptions};
use std::sync::Arc;

pub struct Data {
    pub units: UnitCollection,
    pub systemd: Arc<dyn SystemdManager>,
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

#[automock]
pub trait CommandContext {
    async fn defer_response(&self) -> Result<()>;
    async fn respond(&self, response: String) -> Result<()>;

    fn get_command_name(&self) -> &str;
    fn get_units(&self) -> &UnitCollection;
    fn get_systemd(&self) -> &dyn SystemdManager;
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

    fn get_units(&self) -> &UnitCollection {
        &self.data().units
    }
    fn get_systemd(&self) -> &dyn SystemdManager {
        self.data().systemd.as_ref()
    }
}

pub fn build_framework(
    guild_id: GuildId,
    command_type: CommandType,
    units: UnitSection,
    systemd: Arc<dyn SystemdManager>,
) -> Framework<Arc<Data>, Error> {
    let data = Arc::from(Data {
        units: units.command_units,
        systemd: systemd.clone(),
    });
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
                    monitor_status(&ctx, &units.status_units, systemd.as_ref()).await;
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
