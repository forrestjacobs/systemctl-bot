mod client;
mod commands;
mod config;
mod status_monitor;
mod systemd;

use anyhow::Result;
use client::{build_framework, start_client};
use config::Config;
use std::sync::Arc;
use systemd::SystemdManagerImpl;

async fn start() -> Result<()> {
    let config = Config::build()?;
    let systemd = Arc::from(SystemdManagerImpl::build().await?);
    let framework = build_framework(config.guild_id, config.command_type, config.units, systemd);

    start_client(config.discord_token, config.application_id, framework).await
}

#[tokio::main]
async fn main() {
    start().await.unwrap();
}
