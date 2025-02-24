use crate::builder::build_commands;
use crate::command::{CommandRunner, UserCommand};
use crate::config::{Command, CommandType, Config};
use crate::status_monitor::StatusMonitor;
use async_trait::async_trait;
use serenity::all::{
    CommandDataOption, CommandDataOptionValue, CommandInteraction, Context,
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    GuildId, Interaction,
};
use shaku::{Component, Interface};
use std::sync::Arc;

#[async_trait]
pub trait Handler: Interface {
    async fn ready(&self, ctx: Context);
    async fn interaction_create(&self, ctx: Context, interaction: Interaction);
}

#[derive(Component)]
#[shaku(interface = Handler)]
pub struct HandlerImpl {
    #[shaku(inject)]
    config: Arc<dyn Config>,
    #[shaku(inject)]
    command_runner: Arc<dyn CommandRunner>,
    #[shaku(inject)]
    status_monitor: Arc<dyn StatusMonitor>,
}

impl HandlerImpl {
    fn get_unit_from_opt(&self, option: &CommandDataOption) -> Option<String> {
        match &option.value {
            CommandDataOptionValue::String(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn parse_command(&self, interaction: &CommandInteraction) -> Option<UserCommand> {
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

#[async_trait]
impl Handler for HandlerImpl {
    async fn ready(&self, ctx: Context) {
        GuildId::new(self.config.guild_id)
            .set_commands(
                &ctx.http,
                build_commands(&self.config.units, &self.config.command_type),
            )
            .await
            .unwrap();

        self.status_monitor.monitor(ctx).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(interaction) = interaction {
            match self.parse_command(&interaction) {
                Some(command) => {
                    interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Defer(
                                CreateInteractionResponseMessage::new(),
                            ),
                        )
                        .await
                        .unwrap();
                    let response_content = match self.command_runner.run(&command).await {
                        Ok(value) => value,
                        Err(value) => value.to_string(),
                    };
                    interaction
                        .create_followup(
                            &ctx.http,
                            CreateInteractionResponseFollowup::new().content(response_content),
                        )
                        .await
                        .unwrap();
                }
                _ => {
                    interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Message(
                                CreateInteractionResponseMessage::new().content("Invalid command"),
                            ),
                        )
                        .await
                        .unwrap();
                }
            }
        }
    }
}
