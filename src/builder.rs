use crate::config::CommandType;
use crate::units::{get_units_with_permissions, Unit, UnitPermission};
use indexmap::IndexMap;
use itertools::Itertools;
use serenity::all::CommandOptionType;
use serenity::builder::{CreateCommand, CreateCommandOption};

struct CommandDescription<'a> {
    permissions: Vec<UnitPermission>,
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
    get_units_with_permissions(units, desc.permissions.iter()).fold(base_option, |option, unit| {
        let alias = unit.strip_suffix(".service").unwrap_or(unit);
        option.add_string_choice(alias, unit)
    })
}

fn create_commands(units: &IndexMap<String, Unit>) -> impl Iterator<Item = CommandDescription<'_>> {
    let commands = [
        CommandDescription {
            permissions: vec![UnitPermission::Start],
            name: "start",
            description: "Start units",
            unit_option_description: "The unit to start",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: vec![UnitPermission::Stop],
            name: "stop",
            description: "Stop units",
            unit_option_description: "The unit to stop",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: vec![UnitPermission::Stop, UnitPermission::Start],
            name: "restart",
            description: "Restarts units",
            unit_option_description: "The unit to restart",
            unit_option_required: true,
        },
        CommandDescription {
            permissions: vec![UnitPermission::Status],
            name: "status",
            description: "Checks units' status",
            unit_option_description: "The unit to check",
            unit_option_required: false,
        },
    ];

    commands.into_iter().filter(|command| {
        get_units_with_permissions(units, command.permissions.iter())
            .peekable()
            .peek()
            .is_some()
    })
}

pub fn build_commands<'a>(
    units: &IndexMap<String, Unit>,
    command_type: &CommandType,
) -> Vec<CreateCommand> {
    match command_type {
        CommandType::Single => {
            let options = create_commands(units)
                .map(|desc| {
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        desc.name,
                        desc.description,
                    )
                    .add_sub_option(create_unit_option(units, &desc))
                })
                .collect_vec();
            let command = CreateCommand::new("systemctl")
                .description("Controls units")
                .set_options(options);
            vec![command]
        }
        CommandType::Multiple => create_commands(units)
            .map(|desc| {
                CreateCommand::new(desc.name)
                    .description(desc.description)
                    .add_option(create_unit_option(units, &desc))
            })
            .collect_vec(),
    }
}
