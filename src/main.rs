mod client;
mod commands;
mod config;
mod status_monitor;
mod systemctl;
mod systemd_status;

use anyhow::Result;
use client::{build_framework, start_client, Data};
use config::Config;
use status_monitor::StatusMonitorImpl;
use std::sync::Arc;
use systemctl::SystemctlImpl;
use systemd_status::SystemdStatusManagerImpl;

async fn start() -> Result<()> {
    let config = Config::build()?;
    let units = config.units;

    let framework = build_framework(
        config.guild_id,
        config.command_type,
        Arc::from(StatusMonitorImpl {
            units: units.status_units,
            systemd_status_manager: Arc::from(SystemdStatusManagerImpl::build().await?),
        }),
        Arc::from(Data {
            units: units.command_units,
            systemctl: Arc::from(SystemctlImpl {}),
        }),
    );

    start_client(config.discord_token, config.application_id, framework).await
}

#[tokio::main]
async fn main() {
    start().await.unwrap();
}
