mod command;
mod config;
mod handler;
mod systemctl;

use crate::config::get_config;
use clap::Parser;
use serenity::client::Client;
use serenity::model::gateway::GatewayIntents;
use serenity::model::id::GuildId;

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "/etc/systemctl-bot.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = get_config(args.config).await.unwrap();

    let mut client = Client::builder(
        config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .event_handler(handler::Handler {
        guild_id: GuildId(config.guild_id),
        units: config.units,
    })
    .application_id(config.application_id)
    .await
    .unwrap();

    client.start().await.unwrap();
}
