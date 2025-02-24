use crate::{
    command::UserCommand,
    config::{Command, CommandType, Config},
};
use serenity::all::{CommandDataOption, CommandDataOptionValue, CommandInteraction};
use shaku::{Component, Interface};
use std::sync::Arc;

pub trait Parser: Interface {
    fn parse(&self, interaction: &CommandInteraction) -> Option<UserCommand>;
}

#[derive(Component)]
#[shaku(interface = Parser)]
pub struct ParserImpl {
    #[shaku(inject)]
    config: Arc<dyn Config>,
}

impl ParserImpl {
    fn get_unit_from_opt(&self, option: &CommandDataOption) -> Option<String> {
        match &option.value {
            CommandDataOptionValue::String(name) => Some(name.clone()),
            _ => None,
        }
    }
}

impl Parser for ParserImpl {
    fn parse(&self, interaction: &CommandInteraction) -> Option<UserCommand> {
        let (name, options) = match self.config.command_type {
            CommandType::Single => {
                let sub = interaction.data.options.get(0)?;
                if let CommandDataOptionValue::SubCommand(opts) = &sub.value {
                    (&sub.name, opts)
                } else {
                    return None;
                }
            }
            CommandType::Multiple => (&interaction.data.name, &interaction.data.options),
        };
        match name.as_str() {
            "start" => Some(UserCommand::Start {
                unit: self.get_unit_from_opt(options.get(0)?)?,
            }),
            "stop" => Some(UserCommand::Stop {
                unit: self.get_unit_from_opt(options.get(0)?)?,
            }),
            "restart" => Some(UserCommand::Restart {
                unit: self.get_unit_from_opt(options.get(0)?)?,
            }),
            "status" => Some(match options.get(0) {
                Some(option) => UserCommand::SingleStatus {
                    unit: self.get_unit_from_opt(option)?,
                },
                None => UserCommand::MultiStatus {
                    units: self.config.units[&Command::Status].clone(),
                },
            }),
            _ => None,
        }
    }
}
