mod config;
mod handler;

use serenity::client::Client;
use serenity::model::id::GuildId;

use crate::config::get_config;

#[tokio::main]
async fn main() {
    let config = get_config();

    let mut client = Client::builder(config.discord_token)
        .event_handler(handler::Handler {
            guild_id: GuildId(config.guild_id),
            services: config.services,
        })
        .application_id(config.application_id)
        .await
        .unwrap();

    client.start().await.unwrap();
}
