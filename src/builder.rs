use crate::config::{CommandType, Unit, UnitPermission};
use indexmap::IndexMap;
use serenity::builder::{
    CreateApplicationCommand, CreateApplicationCommandOption, CreateApplicationCommands,
};
use serenity::model::interactions::application_command::ApplicationCommandOptionType;

struct Command<'a> {
    name: &'a str,
    description: &'a str,
    units: Vec<&'a str>,
    units_description: &'a str,
    units_required: bool,
}

fn setup_unit_option<'a>(
    builder: &'a mut CreateApplicationCommandOption,
    command: &Command<'_>,
) -> &'a mut CreateApplicationCommandOption {
    builder
        .name("unit")
        .kind(ApplicationCommandOptionType::String)
        .description(command.units_description)
        .required(command.units_required);
    for unit in &command.units {
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

fn create_commands(units: &IndexMap<String, Unit>) -> Vec<Command> {
    let mut commands = Vec::new();

    let startable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Start)
    });
    if !startable_units.is_empty() {
        commands.push(Command {
            name: "start",
            description: "Start units",
            units: startable_units,
            units_description: "The unit to start",
            units_required: true,
        });
    }

    let stoppable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Stop)
    });
    if !stoppable_units.is_empty() {
        commands.push(Command {
            name: "stop",
            description: "Stops units",
            units: stoppable_units,
            units_description: "The unit to stop",
            units_required: true,
        });
    }

    let restartable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Stop)
            && unit.permissions.contains(&UnitPermission::Start)
    });
    if !restartable_units.is_empty() {
        commands.push(Command {
            name: "restart",
            description: "Restarts units",
            units: restartable_units,
            units_description: "The unit to restart",
            units_required: true,
        });
    }

    let checkable_units = get_filtered_units(units, |unit| {
        unit.permissions.contains(&UnitPermission::Status)
    });
    if !checkable_units.is_empty() {
        commands.push(Command {
            name: "status",
            description: "Checks units' status",
            units: checkable_units,
            units_description: "The unit to check",
            units_required: false,
        });
    }

    commands
}

fn build_single_command<'a>(
    commands: Vec<Command>,
    builder: &'a mut CreateApplicationCommand,
) -> &'a mut CreateApplicationCommand {
    builder.name("systemctl").description("Controls units");
    for command in commands {
        builder.create_option(|sub| {
            sub.name(command.name)
                .description(command.description)
                .kind(ApplicationCommandOptionType::SubCommand)
                .create_sub_option(|opt| setup_unit_option(opt, &command))
        });
    }
    builder
}

fn build_multiple_commands<'a>(
    commands: Vec<Command>,
    builder: &'a mut CreateApplicationCommands,
) -> &'a mut CreateApplicationCommands {
    for command in commands {
        builder.create_application_command(|ac| {
            ac.name(command.name)
                .description(command.description)
                .create_option(|opt| setup_unit_option(opt, &command))
        });
    }
    builder
}

pub fn build_commands<'a>(
    units: &IndexMap<String, Unit>,
    command_type: &CommandType,
    builder: &'a mut CreateApplicationCommands,
) -> &'a mut CreateApplicationCommands {
    let commands = create_commands(units);

    match command_type {
        CommandType::Single => {
            builder.create_application_command(|command| build_single_command(commands, command))
        }
        CommandType::Multiple => build_multiple_commands(commands, builder),
    }
}
