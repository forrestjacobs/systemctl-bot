mod command;
mod command_builder;
mod command_handler;
mod config;
mod status_handler;
mod systemd;

use std::sync::Arc;

use serenity::all::{Client, GatewayIntents};

use crate::{
    command_builder::CommandBuilder, command_handler::CommandHandler,
    status_handler::StatusHandler, systemd::Systemd,
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
    .event_handler(CommandBuilder {
        config: Arc::clone(&config),
    })
    .event_handler(CommandHandler {
        config: Arc::clone(&config),
        systemd: Arc::clone(&systemd),
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
