use crate::config::CommandType;
use crate::units::{UnitPermissions, Units, UnitsTrait};
use itertools::Itertools;
use serenity::all::{CommandOptionType, CommandOptionType::SubCommand};
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

fn get_commands(units: &Units) -> impl Iterator<Item = Command<'_, impl Iterator<Item = &'_ str>>> {
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
        let mut units = units.with_permissions(description.permissions).peekable();
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
                SubCommand,
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

pub fn build_commands(units: &Units, command_type: &CommandType) -> Vec<CreateCommand> {
    let commands = get_commands(units);
    match command_type {
        CommandType::Single => vec![create_single_command(commands)],
        CommandType::Multiple => create_commands(commands),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::to_value;
    use CommandOptionType::String;

    fn get_unit_fixture() -> Units {
        Units::from([
            ("000.service".to_string(), UnitPermissions::empty()),
            ("001.service".to_string(), UnitPermissions::Status),
            ("010.service".to_string(), UnitPermissions::Stop),
            (
                "011.service".to_string(),
                UnitPermissions::Stop | UnitPermissions::Status,
            ),
            ("100.service".to_string(), UnitPermissions::Start),
            (
                "101.service".to_string(),
                UnitPermissions::Start | UnitPermissions::Status,
            ),
            (
                "110.service".to_string(),
                UnitPermissions::Start | UnitPermissions::Stop,
            ),
            ("111.service".to_string(), UnitPermissions::all()),
        ])
    }

    fn get_start_option_fixture() -> CreateCommandOption {
        CreateCommandOption::new(String, "unit", "The unit to start")
            .required(true)
            .add_string_choice("100", "100.service")
            .add_string_choice("101", "101.service")
            .add_string_choice("110", "110.service")
            .add_string_choice("111", "111.service")
    }

    fn get_stop_option_fixture() -> CreateCommandOption {
        CreateCommandOption::new(String, "unit", "The unit to stop")
            .required(true)
            .add_string_choice("010", "010.service")
            .add_string_choice("011", "011.service")
            .add_string_choice("110", "110.service")
            .add_string_choice("111", "111.service")
    }

    fn get_restart_option_fixture() -> CreateCommandOption {
        CreateCommandOption::new(String, "unit", "The unit to restart")
            .required(true)
            .add_string_choice("110", "110.service")
            .add_string_choice("111", "111.service")
    }

    fn get_status_option_fixture() -> CreateCommandOption {
        CreateCommandOption::new(String, "unit", "The unit to check")
            .add_string_choice("001", "001.service")
            .add_string_choice("011", "011.service")
            .add_string_choice("101", "101.service")
            .add_string_choice("111", "111.service")
    }

    #[test]
    fn build_single_command() {
        assert_eq!(
            to_value(&build_commands(&get_unit_fixture(), &CommandType::Single)).unwrap(),
            to_value(&[CreateCommand::new("systemctl")
                .description("Controls units")
                .set_options(vec![
                    CreateCommandOption::new(SubCommand, "start", "Start units",)
                        .add_sub_option(get_start_option_fixture()),
                    CreateCommandOption::new(SubCommand, "stop", "Stop units",)
                        .add_sub_option(get_stop_option_fixture()),
                    CreateCommandOption::new(SubCommand, "restart", "Restarts units",)
                        .add_sub_option(get_restart_option_fixture()),
                    CreateCommandOption::new(SubCommand, "status", "Checks units' status",)
                        .add_sub_option(get_status_option_fixture())
                ])])
            .unwrap()
        );
    }

    #[test]
    fn build_multiple_commands() {
        assert_eq!(
            to_value(&build_commands(&get_unit_fixture(), &CommandType::Multiple)).unwrap(),
            to_value(&[
                CreateCommand::new("start")
                    .description("Start units",)
                    .add_option(get_start_option_fixture()),
                CreateCommand::new("stop")
                    .description("Stop units",)
                    .add_option(get_stop_option_fixture()),
                CreateCommand::new("restart")
                    .description("Restarts units",)
                    .add_option(get_restart_option_fixture()),
                CreateCommand::new("status")
                    .description("Checks units' status",)
                    .add_option(get_status_option_fixture())
            ])
            .unwrap()
        );
    }
}
