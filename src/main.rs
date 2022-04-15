mod command;
mod config;
mod handler;
mod systemctl;

use crate::config::get_config;
use serenity::client::bridge::gateway::GatewayIntents;
use serenity::client::Client;
use serenity::model::id::GuildId;

#[tokio::main]
async fn main() {
    let config = get_config().unwrap();

    let mut client = Client::builder(config.discord_token)
        .intents(GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES)
        .event_handler(handler::Handler {
            guild_id: GuildId(config.guild_id),
            services: config.services,
        })
        .application_id(config.application_id)
        .await
        .unwrap();

    client.start().await.unwrap();
}
