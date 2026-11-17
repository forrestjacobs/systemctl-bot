mod command;
mod command_builder;
mod command_handler;
mod config;
mod status_handler;
mod systemd;

use serenity::all::{Client, GatewayIntents};

use crate::{
    command_builder::CommandBuilder, command_handler::CommandHandler,
    status_handler::StatusHandler, systemd::Systemd,
};

#[tokio::main]
async fn main() {
    let config = config::build();
    let systemd = Box::leak(Box::new(Systemd::new().await));

    let mut builder = Client::builder(
        config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .application_id(config.application_id);

    let command_units = config.units.command_units;
    if command_units.values().any(|units| !units.is_empty()) {
        let command_units = Box::leak(Box::new(command_units));
        builder = builder
            .event_handler(CommandBuilder {
                command_type: config.command_type.clone(),
                guild_id: config.guild_id,
                units: command_units,
            })
            .event_handler(CommandHandler {
                command_type: config.command_type,
                systemd,
                units: command_units,
            });
    }

    let status_units = config.units.status_units;
    if !status_units.is_empty() {
        builder = builder.event_handler(StatusHandler {
            systemd,
            units: status_units,
        });
    }

    builder.await.unwrap().start().await.unwrap();
}
