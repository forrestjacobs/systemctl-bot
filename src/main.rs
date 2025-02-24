mod builder;
mod command;
mod config;
mod handler;
mod status_monitor;
mod systemctl;
mod systemd_status;

use async_trait::async_trait;
use command::CommandRunnerImpl;
use config::{Config, ConfigImpl};
use handler::{Handler, HandlerImpl};
use serenity::all::{
    ApplicationId, Client, Context, EventHandler, GatewayIntents, Interaction, Ready,
};
use shaku::{module, HasComponent};
use status_monitor::StatusMonitorImpl;
use std::sync::Arc;
use systemctl::SystemctlImpl;
use systemd_status::SystemdStatusManagerImpl;

module! {
    RootModule {
        components = [CommandRunnerImpl, ConfigImpl, StatusMonitorImpl, SystemctlImpl, SystemdStatusManagerImpl, HandlerImpl],
        providers = [],
    }
}

struct HandlerWrapper(Arc<dyn Handler>);

#[async_trait]
impl EventHandler for HandlerWrapper {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        self.0.ready(ctx).await;
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.0.interaction_create(ctx, interaction).await;
    }
}

#[tokio::main]
async fn main() {
    let module = RootModule::builder()
        .with_component_parameters::<SystemdStatusManagerImpl>(
            systemd_status::make_params().await.unwrap(),
        )
        .build();

    let config: &dyn Config = module.resolve_ref();
    let handler: Arc<dyn Handler> = module.resolve();

    let mut client = Client::builder(
        &config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .event_handler(HandlerWrapper(handler))
    .application_id(ApplicationId::new(config.application_id))
    .await
    .unwrap();

    client.start().await.unwrap();
}
