mod client;
mod commands;
mod config;
mod status_monitor;
mod systemd;

use anyhow::Result;
use client::{build_framework, start_client, Data};
use config::Config;
use status_monitor::StatusMonitorImpl;
use std::sync::Arc;
use systemd::SystemdManagerImpl;

async fn start() -> Result<()> {
    let config = Config::build()?;
    let systemd = Arc::from(SystemdManagerImpl::build().await?);

    let framework = build_framework(
        config.guild_id,
        config.command_type,
        StatusMonitorImpl {
            units: config.units.status_units,
            systemd: systemd.clone(),
        },
        Arc::from(Data {
            units: config.units.command_units,
            systemd: systemd,
        }),
    );

    start_client(config.discord_token, config.application_id, framework).await
}

#[tokio::main]
async fn main() {
    start().await.unwrap();
}
