mod client;
mod commands;
mod config;
mod status_monitor;
mod systemctl;
mod systemd_status;

use anyhow::Result;
use client::{build_framework, Data};
use config::Config;
use poise::serenity_prelude::{Client, GatewayIntents};
use status_monitor::StatusMonitorImpl;
use std::sync::Arc;
use systemctl::SystemctlImpl;
use systemd_status::SystemdStatusManagerImpl;
use tokio::spawn;

async fn start() -> Result<()> {
    let systemd_status_manager_handle = spawn(SystemdStatusManagerImpl::build());

    let config = Config::build()?;
    let units = Arc::from(config.units);

    let systemd_status_manager = Arc::from(systemd_status_manager_handle.await??);

    let framework = build_framework(
        config.guild_id,
        config.command_type,
        Arc::from(StatusMonitorImpl {
            units: units.clone(),
            systemd_status_manager: systemd_status_manager.clone(),
        }),
        Arc::from(Data {
            units: units.clone(),
            systemctl: Arc::from(SystemctlImpl {}),
            systemd_status_manager: systemd_status_manager.clone(),
        }),
    );

    Client::builder(
        config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .framework(framework)
    .application_id(config.application_id)
    .await?
    .start()
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    start().await.unwrap();
}
