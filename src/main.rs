mod builder;
mod command;
mod config;
mod handler;
mod systemctl;
mod systemd_status;

use command::CommandRunnerImpl;
use config::{Config, ConfigImpl};
use handler::{Handler, HandlerImpl};
use serenity::async_trait;
use serenity::client::Client;
use serenity::client::Context;
use serenity::client::EventHandler;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::GatewayIntents;
use serenity::model::gateway::Ready;
use shaku::{module, HasComponent};
use std::sync::Arc;
use systemd_status::SystemdStatusManagerImpl;
use systemd_status::SystemdStatusManagerImplParameters;

module! {
    RootModule {
        components = [CommandRunnerImpl, ConfigImpl, SystemdStatusManagerImpl, HandlerImpl],
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
        .with_component_parameters::<SystemdStatusManagerImpl>(SystemdStatusManagerImplParameters {
            client: systemd_status::get_client().await.unwrap(),
        })
        .build();

    let config: &dyn Config = module.resolve_ref();
    let handler: Arc<dyn Handler> = module.resolve();

    let mut client = Client::builder(
        &config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .event_handler(HandlerWrapper(handler))
    .application_id(config.application_id)
    .await
    .unwrap();

    client.start().await.unwrap();
}
