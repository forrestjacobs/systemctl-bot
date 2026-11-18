mod command;
mod command_builder;
mod command_handler;
mod config;
mod status_handler;
mod systemd;
mod unit;

use serenity::all::{Client, ClientBuilder, GatewayIntents};

use crate::{
    command_builder::CommandBuilder,
    command_handler::CommandHandler,
    config::SystemctlBotConfig,
    status_handler::StatusHandler,
    systemd::{Systemd, SystemdImpl},
    unit::{get_command_units, get_status_units},
};

fn make_builder<S>(config: SystemctlBotConfig, systemd: &'static S) -> ClientBuilder
where
    S: Systemd + Sync,
{
    let mut builder = Client::builder(
        config.discord_token,
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES,
    )
    .application_id(config.application_id);

    let units = Box::leak(Box::new(config.units));
    let command_units = get_command_units(units);
    if command_units
        .values()
        .any(|unit_list| !unit_list.is_empty())
    {
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

    let status_units = get_status_units(units);
    if !status_units.is_empty() {
        builder = builder.event_handler(StatusHandler {
            systemd,
            units: status_units,
        });
    }
    builder
}

#[tokio::main]
async fn main() {
    let config = config::build();
    let systemd = Box::leak(Box::new(SystemdImpl::new().await));
    make_builder(config, systemd)
        .await
        .expect("Unable to build the Discord client")
        .start()
        .await
        .expect("Unable to start the Discord client");
}

#[cfg(test)]
mod tests {
    use core::mem::transmute;
    use core::ptr;
    use std::collections::{HashMap, HashSet};

    use serenity::all::{ApplicationId, GuildId};

    use crate::{
        command::Command,
        config::{CommandType, Unit, UnitPermission},
        systemd::MockSystemd,
    };

    use super::*;

    #[test]
    fn basic_builder() {
        let config = SystemctlBotConfig {
            application_id: ApplicationId::new(88_888_888),
            guild_id: GuildId::new(12_345_678),
            discord_token: "88888888.88888888.88888888".to_owned(),
            command_type: CommandType::Single,
            units: vec![
                Unit {
                    name: "restartable.service".to_owned(),
                    permissions: HashSet::from([UnitPermission::Start, UnitPermission::Stop]),
                },
                Unit {
                    name: "reportable.service".to_owned(),
                    permissions: HashSet::from([UnitPermission::Status]),
                },
            ],
        };
        let systemd = Box::leak(Box::new(MockSystemd::new()));
        let builder = make_builder(config, systemd);
        assert_eq!(builder.get_token(), "Bot 88888888.88888888.88888888");
        assert_eq!(
            builder.get_application_id(),
            Some(ApplicationId::new(88_888_888))
        );

        let handler = builder.get_event_handlers();
        assert_eq!(handler.len(), 3);

        let command_units = HashMap::from([
            (Command::Start, vec!["restartable.service"]),
            (Command::Stop, vec!["restartable.service"]),
            (Command::Restart, vec!["restartable.service"]),
        ]);

        // SAFETY: The test should panic if this is wrong
        let (command_builder, _): (&CommandBuilder, *const ()) =
            unsafe { transmute(handler[0].as_ref()) };
        assert_eq!(command_builder.command_type, CommandType::Single);
        assert_eq!(command_builder.guild_id, GuildId::new(12_345_678));
        assert_eq!(command_builder.units, &command_units);

        // SAFETY: The test should panic if this is wrong
        let (command_handler, _): (&CommandHandler<MockSystemd>, *const ()) =
            unsafe { transmute(handler[1].as_ref()) };
        assert_eq!(command_handler.command_type, CommandType::Single);
        assert!(ptr::eq(command_handler.systemd, systemd));
        assert_eq!(command_handler.units, &command_units);

        // SAFETY: The test should panic if this is wrong
        let (status_handler, _): (&StatusHandler<MockSystemd>, *const ()) =
            unsafe { transmute(handler[2].as_ref()) };
        assert!(ptr::eq(status_handler.systemd, systemd));
        assert_eq!(status_handler.units, vec!["reportable.service"]);
    }
}
