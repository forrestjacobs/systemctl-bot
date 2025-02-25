use crate::{
    command::{self, Data},
    config::{CommandType, Config},
    status_monitor::StatusMonitor,
};
use async_trait::async_trait;
use poise::{
    samples::register_in_guild,
    serenity_prelude::all::{Client, Error, GatewayIntents},
    Command, Framework, FrameworkOptions,
};
use shaku::{Component, Interface};
use std::sync::Arc;

#[async_trait]
pub trait ClientBuilder: Interface {
    async fn build(&self) -> Result<Client, Error>;
}

#[derive(Component)]
#[shaku(interface = ClientBuilder)]
pub struct ClientBuilderImpl {
    #[shaku(inject)]
    config: Arc<dyn Config>,
    #[shaku(inject)]
    status_monitor: Arc<dyn StatusMonitor>,
    #[shaku(inject)]
    data: Arc<dyn Data>,
}

#[async_trait]
impl ClientBuilder for ClientBuilderImpl {
    async fn build(&self) -> Result<Client, Error> {
        let guild_id = self.config.guild_id;
        let status_monitor = self.status_monitor.clone();
        let data = self.data.clone();
        let commands = vec![
            command::start(),
            command::stop(),
            command::restart(),
            command::status(),
        ];
        let commands = match self.config.command_type {
            CommandType::Multiple => commands,
            CommandType::Single => vec![Command {
                name: "systemctl".into(),
                subcommands: commands,
                ..Default::default()
            }],
        };
        let framework = Framework::builder()
            .options(FrameworkOptions {
                commands,
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
            .build();

        Client::builder(
            &self.config.discord_token,
            GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
        )
        .framework(framework)
        .application_id(self.config.application_id)
        .await
    }
}
