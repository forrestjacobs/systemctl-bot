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

    start_client(config.discord_token, config.application_id, framework).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    start().await.unwrap();
}
