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

    let systemctl = Arc::new(Systemctl);

    let systemd_status_manager = Arc::new(systemd_status::build().await.unwrap());

    let parser = Arc::new(Parser {
        config: Arc::clone(&config),
    });
    let command_runner = Arc::new(CommandRunner {
        config: Arc::clone(&config),
        systemctl,
        systemd_status_manager: Arc::clone(&systemd_status_manager),
    });

    let status_monitor = Arc::new(StatusMonitor {
        config: Arc::clone(&config),
        systemd_status_manager,
    });

    Client::builder(
        config.discord_token.clone(),
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .application_id(ApplicationId::new(config.application_id))
    .event_handler(Handler {
        config,
        parser,
        command_runner,
        status_monitor,
    })
    .await
    .unwrap()
    .start()
    .await
    .unwrap();
}
