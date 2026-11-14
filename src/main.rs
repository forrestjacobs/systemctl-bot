mod builder;
mod command;
mod config;
mod handler;
mod parser;
mod status_handler;
mod systemd;

use std::sync::Arc;

use serenity::all::{Client, GatewayIntents};

use crate::{
    command::CommandRunner, handler::Handler, parser::Parser, status_handler::StatusHandler,
    systemd::Systemd,
};

#[tokio::main]
async fn main() {
    let config = Arc::new(config::build());
    let systemd = Arc::new(Systemd::new().await);

    Client::builder(
        config.discord_token.clone(),
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .application_id(config.application_id)
    .event_handler(Handler {
        config: Arc::clone(&config),
        parser: Parser {
            config: Arc::clone(&config),
        },
        command_runner: CommandRunner {
            config: Arc::clone(&config),
            systemd: Arc::clone(&systemd),
        },
    })
    .event_handler(StatusHandler {
        systemd,
        units: config.units.status_units.clone(),
    })
    .await
    .unwrap()
    .start()
    .await
    .unwrap();
}
