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

struct Command<'a, U: Iterator<Item = &'a str>> {
    description: CommandDescription<'a>,
    units: U,
}

fn create_unit_option<'a, U>(command: Command<'a, U>) -> CreateCommandOption
where
    U: Iterator<Item = &'a str>,
{
    let base_option = CreateCommandOption::new(
        CommandOptionType::String,
        "unit",
        command.description.unit_option_description,
    )
    .required(command.description.unit_option_required);
    command.units.fold(base_option, |option, unit| {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        option.add_string_choice(alias, unit)
    })
}

fn get_commands(
    units: &IndexMap<String, Unit>,
) -> impl Iterator<Item = Command<'_, impl Iterator<Item = &'_ str>>> {
    let descriptions = [
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

    descriptions.into_iter().filter_map(|description| {
        let mut units = get_units_with_permissions(units, description.permissions).peekable();
        match units.peek() {
            Some(_) => Some(Command { description, units }),
            None => None,
        }
    })
}

fn create_single_command<'a, U, I>(commands: I) -> CreateCommand
where
    U: Iterator<Item = &'a str>,
    I: Iterator<Item = Command<'a, U>>,
{
    let options = commands
        .map(|command| {
            CreateCommandOption::new(
                CommandOptionType::SubCommand,
                command.description.name,
                command.description.description,
            )
            .add_sub_option(create_unit_option(command))
        })
        .collect_vec();
    CreateCommand::new("systemctl")
        .description("Controls units")
        .set_options(options)
}

fn create_commands<'a, U, I>(commands: I) -> Vec<CreateCommand>
where
    U: Iterator<Item = &'a str>,
    I: Iterator<Item = Command<'a, U>>,
{
    commands
        .map(|command| {
            CreateCommand::new(command.description.name)
                .description(command.description.description)
                .add_option(create_unit_option(command))
        })
        .collect_vec()
}

pub fn build_commands(
    units: &IndexMap<String, Unit>,
    command_type: &CommandType,
) -> Vec<CreateCommand> {
    let commands = get_commands(units);
    match command_type {
        CommandType::Single => vec![create_single_command(commands)],
        CommandType::Multiple => create_commands(commands),
    }
}
