use serenity::all::{
    CommandOptionType, Context, CreateCommand, CreateCommandOption, EventHandler, GuildId, Ready,
};
use serenity::async_trait;

use crate::{
    command::Command,
    config::{CommandType, CommandUnits},
};

fn build_commands(command_type: &CommandType, units: &CommandUnits) -> Vec<CreateCommand> {
    let command_iter = Command::VALUES
        .iter()
        .filter(|command| !&units[command].is_empty())
        .map(|command| {
            let name = command.to_string();

            let description = {
                let mut capitalized_name = name.clone();
                capitalized_name[0..1].make_ascii_uppercase();
                format!("{} units", capitalized_name)
            };

            let option = {
                let base = CreateCommandOption::new(
                    CommandOptionType::String,
                    "unit",
                    format!("The unit to {}", name),
                )
                .required(true);
                units[command].iter().fold(base, |option, unit| {
                    let alias = unit.strip_suffix(".service").unwrap_or(unit);
                    option.add_string_choice(alias, unit)
                })
            };

            (name, description, option)
        });

    match &command_type {
        CommandType::Single => {
            let options = command_iter
                .map(|(name, description, option)| {
                    CreateCommandOption::new(CommandOptionType::SubCommand, name, description)
                        .add_sub_option(option)
                })
                .collect();
            let command = CreateCommand::new("systemctl")
                .description("Controls units")
                .set_options(options);
            vec![command]
        }
        CommandType::Multiple => command_iter
            .map(|(name, description, option)| {
                CreateCommand::new(name)
                    .description(description)
                    .add_option(option)
            })
            .collect(),
    }
}

pub struct CommandBuilder {
    pub command_type: CommandType,
    pub guild_id: GuildId,
    pub units: &'static CommandUnits,
}

#[async_trait]
impl EventHandler for CommandBuilder {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        self.guild_id
            .set_commands(&ctx.http, build_commands(&self.command_type, self.units))
            .await
            .expect("Unable to set commands");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serenity::json::to_value;

    use super::*;

    fn units_fixture() -> CommandUnits {
        HashMap::from([
            (
                Command::Start,
                vec![
                    "startable.service".to_string(),
                    "restartable.service".to_string(),
                ],
            ),
            (
                Command::Stop,
                vec![
                    "stoppable.service".to_string(),
                    "restartable.service".to_string(),
                ],
            ),
            (Command::Restart, vec!["restartable.service".to_string()]),
        ])
    }

    #[test]
    fn build_single_command() {
        let units = units_fixture();
        assert_eq!(
            to_value(build_commands(&CommandType::Single, &units)).unwrap(),
            to_value(vec![CreateCommand::new("systemctl")
                .description("Controls units")
                .add_option(
                    CreateCommandOption::new(CommandOptionType::SubCommand, "start", "Start units")
                        .add_sub_option(
                            CreateCommandOption::new(
                                CommandOptionType::String,
                                "unit",
                                "The unit to start",
                            )
                            .required(true)
                            .add_string_choice("startable", "startable.service")
                            .add_string_choice("restartable", "restartable.service")
                        )
                )
                .add_option(
                    CreateCommandOption::new(CommandOptionType::SubCommand, "stop", "Stop units")
                        .add_sub_option(
                            CreateCommandOption::new(
                                CommandOptionType::String,
                                "unit",
                                "The unit to stop",
                            )
                            .required(true)
                            .add_string_choice("stoppable", "stoppable.service")
                            .add_string_choice("restartable", "restartable.service")
                        )
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "restart",
                        "Restart units"
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "unit",
                            "The unit to restart",
                        )
                        .required(true)
                        .add_string_choice("restartable", "restartable.service")
                    )
                )])
            .unwrap()
        );
    }

    #[test]
    fn build_multiple_commands() {
        let units = units_fixture();
        assert_eq!(
            to_value(build_commands(&CommandType::Multiple, &units)).unwrap(),
            to_value(vec![
                CreateCommand::new("start")
                .description("Start units")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "unit",
                        "The unit to start",
                    )
                    .required(true)
                    .add_string_choice("startable", "startable.service")
                    .add_string_choice("restartable", "restartable.service")
                ),
                CreateCommand::new("stop")
                .description("Stop units")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "unit",
                        "The unit to stop",
                    )
                    .required(true)
                    .add_string_choice("stoppable", "stoppable.service")
                    .add_string_choice("restartable", "restartable.service")
                ),
                CreateCommand::new("restart")
                .description("Restart units")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "unit",
                        "The unit to restart",
                    )
                    .required(true)
                    .add_string_choice("restartable", "restartable.service")
                ),
            ])
            .unwrap()
        );
    }
}
