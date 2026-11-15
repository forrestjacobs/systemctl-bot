use crate::{
    command::Command,
    config::{CommandType, SystemctlBotConfig},
};
use serenity::all::{CommandDataOption, CommandDataOptionValue, CommandInteraction};
use std::sync::Arc;

pub struct Parser {
    pub config: Arc<SystemctlBotConfig>,
}

impl Parser {
    fn get_unit_from_opt(&self, option: &CommandDataOption) -> Option<String> {
        match &option.value {
            CommandDataOptionValue::String(name) => Some(name.clone()),
            _ => None,
        }
    }

    pub fn parse(&self, interaction: &CommandInteraction) -> Option<(Command, String)> {
        let (name, options) = match self.config.command_type {
            CommandType::Single => {
                let sub = interaction.data.options.first()?;
                if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                    (&sub.name, opts)
                } else {
                    return None;
                }
            }
            CommandType::Multiple => (&interaction.data.name, &interaction.data.options),
        };
        Some((
            match name.as_str() {
                "start" => Command::Start,
                "stop" => Command::Stop,
                "restart" => Command::Restart,
                _ => return None,
            },
            self.get_unit_from_opt(options.first()?)?,
        ))
    }
}
