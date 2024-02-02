use crate::command::UserCommand;
use crate::config::CommandType;
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};

fn get_string(option: &CommandDataOption) -> Option<String> {
    match &option.resolved {
        Some(CommandDataOptionValue::String(name)) => Some(name.to_owned()),
        _ => None,
    }
}

pub fn parse_command(
    command_type: &CommandType,
    interaction: &ApplicationCommandInteraction,
) -> Option<UserCommand> {
    let (name, options) = match command_type {
        CommandType::Single => {
            let sub = interaction.data.options.get(0)?;
            (&sub.name, &sub.options)
        }
        CommandType::Multiple => (&interaction.data.name, &interaction.data.options),
    };
    match name.as_str() {
        "start" => Some(UserCommand::Start {
            unit: get_string(options.get(0)?)?,
        }),
        "stop" => Some(UserCommand::Stop {
            unit: get_string(options.get(0)?)?,
        }),
        "restart" => Some(UserCommand::Restart {
            unit: get_string(options.get(0)?)?,
        }),
        "status" => Some(match options.get(0) {
            Some(option) => UserCommand::SingleStatus {
                unit: get_string(option)?,
            },
            None => UserCommand::MultiStatus,
        }),
        _ => None,
    }
}
