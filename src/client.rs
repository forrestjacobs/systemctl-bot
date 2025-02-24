use crate::{config::Config, handler::Handler};
use async_trait::async_trait;
use serenity::{
    all::{ApplicationId, Context, EventHandler, GatewayIntents, Interaction, Ready},
    Client, Error,
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
    handler: Arc<dyn Handler>,
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

#[async_trait]
impl ClientBuilder for ClientBuilderImpl {
    async fn build(&self) -> Result<Client, Error> {
        Client::builder(
            &self.config.discord_token,
            GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
        )
        .event_handler(HandlerWrapper(self.handler.to_owned()))
        .application_id(ApplicationId::new(self.config.application_id))
        .await
    }
}
