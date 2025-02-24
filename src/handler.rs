use crate::builder::build_commands;
use crate::command::{CommandRunner, UserCommand};
use crate::config::{Command, CommandType, Config};
use crate::systemd_status::SystemdStatusManager;
use async_trait::async_trait;
use futures::future::join_all;
use futures::StreamExt;
use serenity::all::{
    ActivityData, CommandDataOption, CommandDataOptionValue, CommandInteraction, Context,
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
    GuildId, Interaction,
};
use shaku::{Component, Interface};
use std::sync::Arc;
use tokio_stream::StreamMap;
use zbus::PropertyStream;

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
    systemd_status_manager: Arc<dyn SystemdStatusManager>,
    #[shaku(inject)]
    command_runner: Arc<dyn CommandRunner>,
}

impl HandlerImpl {
    async fn update_activity_stream(
        &self,
    ) -> Result<StreamMap<&str, PropertyStream<'_, String>>, zbus::Error> {
        let streams = self.config.units[&Command::Status]
            .iter()
            .map(|u| self.systemd_status_manager.status_stream(u));
        let streams = join_all(streams)
            .await
            .into_iter()
            .collect::<Result<Vec<PropertyStream<String>>, zbus::Error>>()?;
        Ok(self.config.units[&Command::Status]
            .iter()
            .map(|u| u.as_str())
            .zip(streams)
            .collect::<StreamMap<&str, PropertyStream<String>>>())
    }

    async fn update_activity(&self, ctx: &Context) {
        let units = self.config.units[&Command::Status]
            .iter()
            .map(|u| u.as_str());
        let statuses = self.systemd_status_manager.statuses(units).await;
        let active_units = statuses
            .into_iter()
            .filter(|(_, status)| status.as_ref().map_or(false, |status| status == "active"))
            .map(|(unit, _)| unit)
            .collect::<Vec<&str>>();

        if active_units.is_empty() {
            ctx.reset_presence();
        } else {
            let activity = ActivityData::playing(active_units.join(", "));
            ctx.set_activity(Some(activity));
        }
    }

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

        let mut stream = self.update_activity_stream().await.unwrap();
        self.update_activity(&ctx).await;
        while stream.next().await.is_some() {
            self.update_activity(&ctx).await;
        }
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
