use crate::config::{CommandType, Unit, UnitPermission};
use indexmap::IndexMap;
use serenity::builder::{CreateApplicationCommandOption, CreateApplicationCommands};
use serenity::model::application::command::CommandOptionType;

struct UnitOption<'a> {
    units: Vec<&'a str>,
    description: &'a str,
    required: bool,
}

fn setup_unit_option<'a>(
    builder: &'a mut CreateApplicationCommandOption,
    unit_option: &UnitOption<'_>,
) -> &'a mut CreateApplicationCommandOption {
    builder
        .name("unit")
        .kind(CommandOptionType::String)
        .description(unit_option.description)
        .required(unit_option.required);
    for unit in &unit_option.units {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        builder.add_string_choice(alias, unit);
    }
    builder
}

fn get_filtered_units<P: Fn(&Unit) -> bool>(
    units: &IndexMap<String, Unit>,
    predicate: P,
) -> Vec<&str> {
    units
        .iter()
        .filter(|(_, unit)| predicate(unit))
        .map(|(name, _)| name.as_str())
        .collect::<Vec<&str>>()
}

fn create_commands<F>(units: &IndexMap<String, Unit>, mut register: F)
where
    F: FnMut(&str, &str, UnitOption),
{
    let startable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Start)
    });
    if !startable_units.is_empty() {
        let option = UnitOption {
            units: startable_units,
            description: "The unit to start",
            required: true,
        };
        register("start", "Start units", option);
    }

    let stoppable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Stop)
    });
    if !stoppable_units.is_empty() {
        let option = UnitOption {
            units: stoppable_units,
            description: "The unit to stop",
            required: true,
        };
        register("stop", "Stops units", option);
    }

    let restartable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Stop)
            && unit.permissions.contains(&UnitPermission::Start)
    });
    if !restartable_units.is_empty() {
        let option = UnitOption {
            units: restartable_units,
            description: "The unit to restart",
            required: true,
        };
        register("restart", "Restarts units", option);
    }

    let checkable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Status)
    });
    if !checkable_units.is_empty() {
        let option = UnitOption {
            units: checkable_units,
            description: "The unit to check",
            required: false,
        };
        register("status", "Checks units' status", option);
    }
}

pub fn build_commands<'a>(
    units: &IndexMap<String, Unit>,
    command_type: &CommandType,
    builder: &'a mut CreateApplicationCommands,
) -> &'a mut CreateApplicationCommands {
    match command_type {
        CommandType::Single => builder.create_application_command(|builder| {
            builder.name("systemctl").description("Controls units");
            create_commands(units, |name, description, unit_option| {
                builder.create_option(|o| {
                    o.name(name)
                        .description(description)
                        .kind(CommandOptionType::SubCommand)
                        .create_sub_option(|opt| setup_unit_option(opt, &unit_option))
                });
            });
            builder
        }),
        CommandType::Multiple => {
            create_commands(units, |name, description, unit_option| {
                builder.create_application_command(|c| {
                    c.name(name)
                        .description(description)
                        .create_option(|opt| setup_unit_option(opt, &unit_option))
                });
            });
            builder
        }
    }
}
