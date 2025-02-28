use crate::{
    command::{self, Data},
    config::CommandType,
    status_monitor::StatusMonitor,
};
use poise::{
    samples::register_in_guild, serenity_prelude::GuildId, Command, Framework, FrameworkOptions,
};
use std::{error::Error, sync::Arc};

pub fn build_framework(
    guild_id: GuildId,
    command_type: CommandType,
    status_monitor: Arc<dyn StatusMonitor>,
    data: Arc<Data>,
) -> Framework<Arc<Data>, Box<dyn Error + Send + Sync>> {
    let commands = vec![
        command::start(),
        command::stop(),
        command::restart(),
        command::status(),
    ];
    let commands = match command_type {
        CommandType::Multiple => commands,
        CommandType::Single => vec![Command {
            name: "systemctl".into(),
            subcommands: commands,
            ..Default::default()
        }],
    };
    Framework::builder()
        .options(FrameworkOptions {
            commands,
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                register_in_guild(&ctx.http, &framework.options().commands, guild_id).await?;
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    status_monitor.monitor(&ctx).await;
                });
                Ok(data)
            })
        })
        .build()
}
