mod builder;
mod command;
mod config;
mod handler;
mod parser;
mod status_monitor;
mod systemctl;
mod systemd_status;

use std::sync::Arc;

use serenity::all::{ApplicationId, Client, GatewayIntents};

use crate::{
    command::CommandRunner, handler::Handler, parser::Parser, status_monitor::StatusMonitor,
    systemctl::Systemctl,
};

#[tokio::main]
async fn main() {
    let config = Arc::new(config::build());

    Client::builder(
        config.discord_token.clone(),
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .application_id(ApplicationId::new(config.application_id))
    .event_handler(Handler {
        config: Arc::clone(&config),
        parser: Parser {
            config: Arc::clone(&config),
        },
        command_runner: CommandRunner {
            config: Arc::clone(&config),
            systemctl: Systemctl,
        },
        status_monitor: StatusMonitor {
            config: Arc::clone(&config),
            systemd_status_manager: systemd_status::build().await.unwrap(),
        },
    })
    .await
    .unwrap()
    .start()
    .await
    .unwrap();
}
