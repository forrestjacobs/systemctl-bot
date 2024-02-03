use crate::config::CommandType;
use crate::units::{get_units_with_permissions, Unit, UnitPermissions};
use indexmap::IndexMap;
use itertools::Itertools;
use serenity::all::CommandOptionType;
use serenity::builder::{CreateCommand, CreateCommandOption};

struct CommandDescription<'a> {
    permissions: UnitPermissions,
    name: &'a str,
    description: &'a str,
    unit_option_description: &'a str,
    unit_option_required: bool,
}

fn create_unit_option(
    units: &IndexMap<String, Unit>,
    desc: &CommandDescription<'_>,
) -> CreateCommandOption {
    let base_option = CreateCommandOption::new(
        CommandOptionType::String,
        "unit",
        desc.unit_option_description,
    )
    .required(desc.unit_option_required);
    get_units_with_permissions(units, desc.permissions).fold(base_option, |option, unit| {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        option.add_string_choice(alias, unit)
    })
}

fn create_commands(units: &IndexMap<String, Unit>) -> impl Iterator<Item = CommandDescription<'_>> {
    let commands = [
        CommandDescription {
            permissions: UnitPermissions::Start,
            name: "start",
            description: "Start units",
            unit_option_description: "The unit to start",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: UnitPermissions::Stop,
            name: "stop",
            description: "Stop units",
            unit_option_description: "The unit to stop",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: UnitPermissions::Stop | UnitPermissions::Start,
            name: "restart",
            description: "Restarts units",
            unit_option_description: "The unit to restart",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: UnitPermissions::Status,
            name: "status",
            description: "Checks units' status",
            unit_option_description: "The unit to check",
            unit_option_required: false,
        },
    ];

    commands.into_iter().filter(|command| {
        get_units_with_permissions(units, command.permissions)
            .peekable()
            .peek()
            .is_some()
    })
}

fn build_single_command(units: &IndexMap<String, Unit>) -> CreateCommand {
    let options = create_commands(units)
        .map(|desc| {
            CreateCommandOption::new(CommandOptionType::SubCommand, desc.name, desc.description)
                .add_sub_option(create_unit_option(units, &desc))
        })
        .collect_vec();
    CreateCommand::new("systemctl")
        .description("Controls units")
        .set_options(options)
}

fn build_multiple_commands(units: &IndexMap<String, Unit>) -> Vec<CreateCommand> {
    create_commands(units)
        .map(|desc| {
            CreateCommand::new(desc.name)
                .description(desc.description)
                .add_option(create_unit_option(units, &desc))
        })
        .collect_vec()
}

pub fn build_commands(
    units: &IndexMap<String, Unit>,
    command_type: &CommandType,
) -> Vec<CreateCommand> {
    match command_type {
        CommandType::Single => vec![build_single_command(units)],
        CommandType::Multiple => build_multiple_commands(units),
    }
}
