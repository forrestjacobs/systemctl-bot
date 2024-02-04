use crate::command::UserCommand;
use crate::config::CommandType;
use serenity::all::{CommandDataOptionValue, CommandInteraction};

fn get_name<'a>(
    command_type: &CommandType,
    interaction: &'a CommandInteraction,
) -> Option<&'a str> {
    match command_type {
        CommandType::Single => Some(interaction.data.options.get(0)?.name.as_str()),
        CommandType::Multiple => Some(interaction.data.name.as_str()),
    }
}

fn get_option_value<'a>(
    command_type: &CommandType,
    interaction: &'a CommandInteraction,
) -> Option<String> {
    let options = match command_type {
        CommandType::Single => match &interaction.data.options.get(0)?.value {
            CommandDataOptionValue::SubCommand(options) => Some(options),
            _ => return None,
        },
        CommandType::Multiple => Some(&interaction.data.options),
    };
    match &options?.get(0)?.value {
        CommandDataOptionValue::String(name) => Some(name.to_owned()),
        _ => None,
    }
}

fn get_command(name: &str, unit: Option<String>) -> Option<UserCommand> {
    match name {
        "start" => Some(UserCommand::Start { unit: unit? }),
        "stop" => Some(UserCommand::Stop { unit: unit? }),
        "restart" => Some(UserCommand::Restart { unit: unit? }),
        "status" => Some(match unit {
            Some(option) => UserCommand::SingleStatus { unit: option },
            None => UserCommand::MultiStatus,
        }),
        _ => None,
    }
}

pub fn parse_command(
    command_type: &CommandType,
    interaction: &CommandInteraction,
) -> Option<UserCommand> {
    let name = get_name(command_type, interaction)?;
    let option_value = get_option_value(command_type, interaction);
    get_command(name, option_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command() {
        let unit_name = "unit-name".to_string();
        let some_unit = Some(unit_name.clone());

        assert_eq!(
            get_command("start", some_unit.clone()),
            Some(UserCommand::Start {
                unit: unit_name.clone()
            })
        );
        assert_eq!(get_command("start", None), None);

        assert_eq!(
            get_command("stop", some_unit.clone()),
            Some(UserCommand::Stop {
                unit: unit_name.clone()
            })
        );
        assert_eq!(get_command("stop", None), None);

        assert_eq!(
            get_command("restart", some_unit.clone()),
            Some(UserCommand::Restart {
                unit: unit_name.clone()
            })
        );
        assert_eq!(get_command("restart", None), None);

        assert_eq!(
            get_command("status", some_unit.clone()),
            Some(UserCommand::SingleStatus {
                unit: unit_name.clone()
            })
        );
        assert_eq!(get_command("status", None), Some(UserCommand::MultiStatus));

        assert_eq!(get_command("invalid", None), None);
    }
}
