use crate::config::{Command, CommandType};
use serenity::all::{CommandOptionType, CreateCommand, CreateCommandOption};
use std::collections::HashMap;

struct UnitOption<'a> {
    units: &'a Vec<String>,
    description: &'a str,
    required: bool,
}

fn setup_unit_option<'a>(unit_option: &'a UnitOption<'_>) -> CreateCommandOption {
    let option =
        CreateCommandOption::new(CommandOptionType::String, "unit", unit_option.description)
            .required(unit_option.required);
    unit_option.units.iter().fold(option, |option, unit| {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        option.add_string_choice(alias, unit)
    })
}

fn create_commands<F>(units: &HashMap<Command, Vec<String>>, mut register: F)
where
    F: FnMut(&str, &str, UnitOption),
{
    let startable_units = &units[&Command::Start];
    if !startable_units.is_empty() {
        let option = UnitOption {
            units: startable_units,
            description: "The unit to start",
            required: true,
        };
        register("start", "Start units", option);
    }

    let stoppable_units = &units[&Command::Stop];
    if !stoppable_units.is_empty() {
        let option = UnitOption {
            units: stoppable_units,
            description: "The unit to stop",
            required: true,
        };
        register("stop", "Stops units", option);
    }

    let restartable_units = &units[&Command::Restart];
    if !restartable_units.is_empty() {
        let option = UnitOption {
            units: restartable_units,
            description: "The unit to restart",
            required: true,
        };
        register("restart", "Restarts units", option);
    }

    let checkable_units = &units[&Command::Status];
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
    units: &HashMap<Command, Vec<String>>,
    command_type: &CommandType,
) -> Vec<CreateCommand> {
    match command_type {
        CommandType::Single => {
            let mut options = Vec::new();
            create_commands(units, |name, description, unit_option| {
                options.push(
                    CreateCommandOption::new(
                        serenity::all::CommandOptionType::SubCommand,
                        name,
                        description,
                    )
                    .add_sub_option(setup_unit_option(&unit_option)),
                );
            });
            let command = CreateCommand::new("systemctl").description("Controls units");
            let command = options
                .into_iter()
                .fold(command, |command, option| command.add_option(option));
            vec![command]
        }
        CommandType::Multiple => {
            let mut commands = Vec::new();
            create_commands(units, |name, description, unit_option| {
                commands.push(
                    CreateCommand::new(name)
                        .description(description)
                        .add_option(setup_unit_option(&unit_option)),
                );
            });
            commands
        }
    }
}
