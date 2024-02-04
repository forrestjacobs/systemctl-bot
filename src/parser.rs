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
) -> Option<&'a str> {
    let options = match command_type {
        CommandType::Single => match &interaction.data.options.get(0)?.value {
            CommandDataOptionValue::SubCommand(options) => Some(options),
            _ => return None,
        },
        CommandType::Multiple => Some(&interaction.data.options),
    };
    match &options?.get(0)?.value {
        CommandDataOptionValue::String(name) => Some(name),
        _ => None,
    }
}

fn get_command<'a>(name: &str, unit: Option<&'a str>) -> Option<UserCommand<'a>> {
    match name {
        "start" => Some(UserCommand::Start(unit?)),
        "stop" => Some(UserCommand::Stop(unit?)),
        "restart" => Some(UserCommand::Restart(unit?)),
        "status" => Some(match unit {
            Some(option) => UserCommand::SingleStatus(option),
            None => UserCommand::MultiStatus,
        }),
        _ => None,
    }
}

pub fn parse_command<'a>(
    command_type: &CommandType,
    interaction: &'a CommandInteraction,
) -> Option<UserCommand<'a>> {
    let name = get_name(command_type, interaction)?;
    let option_value = get_option_value(command_type, interaction);
    get_command(name, option_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command() {
        let unit_name = "unit-name";

        assert_eq!(
            get_command("start", Some(unit_name)),
            Some(UserCommand::Start(unit_name))
        );
        assert_eq!(get_command("start", None), None);

        assert_eq!(
            get_command("stop", Some(unit_name)),
            Some(UserCommand::Stop(unit_name))
        );
        assert_eq!(get_command("stop", None), None);

        assert_eq!(
            get_command("restart", Some(unit_name)),
            Some(UserCommand::Restart(unit_name))
        );
        assert_eq!(get_command("restart", None), None);

        assert_eq!(
            get_command("status", Some(unit_name)),
            Some(UserCommand::SingleStatus(unit_name))
        );
        assert_eq!(get_command("status", None), Some(UserCommand::MultiStatus));

        assert_eq!(get_command("invalid", None), None);
    }
}
