mod builder;
mod command;
mod config;
mod handler;
mod parser;
mod status_monitor;
mod systemctl;
mod systemd_status;
mod units;

use crate::config::get_config;
use clap::Parser;
use handler::Handler;
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
    let config = get_config(args.config).unwrap();

    let guild_id = GuildId(config.guild_id);
    let handler = Handler::new(guild_id, config.command_type, config.units)
        .await
        .unwrap();

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;
    let mut client = Client::builder(config.discord_token, intents)
        .event_handler(handler)
        .application_id(config.application_id)
        .await
        .unwrap();

    client.start().await.unwrap();
}
