use crate::config::{Unit, UnitPermission};
use indexmap::IndexMap;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommandOption};
use serenity::model::interactions::application_command::ApplicationCommandOptionType;

fn setup_unit_option<'a>(
    command: &'a mut CreateApplicationCommandOption,
    units: Vec<&String>,
) -> &'a mut CreateApplicationCommandOption {
    command
        .name("unit")
        .kind(ApplicationCommandOptionType::String);
    for unit in units {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        command.add_string_choice(alias, unit);
    }
    command
}

fn with_filtered_units<P: Fn(&Unit) -> bool, F: FnOnce(Vec<&String>)>(
    units: &IndexMap<String, Unit>,
    predicate: P,
    f: F,
) {
    if units.values().any(&predicate) {
        let units = units
            .iter()
            .filter(|(_, unit)| predicate(unit))
            .map(|(name, _)| name)
            .collect::<Vec<&String>>();
        f(units);
    }
}

pub fn build_command<'a>(
    units: &IndexMap<String, Unit>,
    command: &'a mut CreateApplicationCommand,
) -> &'a mut CreateApplicationCommand {
    command.name("systemctl").description("Controls units");
    with_filtered_units(
        units,
        |unit| unit.permissions.contains(&UnitPermission::Start),
        |units| {
            command.create_option(|sub| {
                sub.name("start")
                    .description("Starts units")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|opt| {
                        setup_unit_option(opt, units)
                            .description("The unit to start")
                            .required(true)
                    })
            });
        },
    );
    with_filtered_units(
        units,
        |unit| unit.permissions.contains(&UnitPermission::Stop),
        |units| {
            command.create_option(|sub| {
                sub.name("stop")
                    .description("Stops units")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|opt| {
                        setup_unit_option(opt, units)
                            .description("The unit to stop")
                            .required(true)
                    })
            });
        },
    );
    with_filtered_units(
        units,
        |unit| {
            unit.permissions.contains(&UnitPermission::Stop)
                && unit.permissions.contains(&UnitPermission::Start)
        },
        |units| {
            command.create_option(|sub| {
                sub.name("restart")
                    .description("Restarts units")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|opt| {
                        setup_unit_option(opt, units)
                            .description("The unit to restart")
                            .required(true)
                    })
            });
        },
    );
    with_filtered_units(
        units,
        |unit| unit.permissions.contains(&UnitPermission::Status),
        |units| {
            command.create_option(|sub| {
                sub.name("status")
                    .description("Checks units' status")
                    .kind(ApplicationCommandOptionType::SubCommand)
                    .create_sub_option(|opt| {
                        setup_unit_option(opt, units).description("The unit to check")
                    })
            });
        },
    );
    command
}
