use crate::builder::build_commands;
use crate::command::UserCommand;
use crate::config::{Command, CommandType, Config};
use crate::systemd_status::{statuses, SystemdStatusManager};
use async_trait::async_trait;
use futures::future::join_all;
use futures::StreamExt;
use serenity::client::Context;
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Activity;
use serenity::model::id::GuildId;
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
        let statuses = statuses(self.systemd_status_manager.as_ref(), units).await;
        let active_units = statuses
            .into_iter()
            .filter(|(_, status)| status.as_ref().map_or(false, |status| status == "active"))
            .map(|(unit, _)| unit)
            .collect::<Vec<&str>>();

        if active_units.is_empty() {
            ctx.reset_presence().await;
        } else {
            let activity = Activity::playing(active_units.join(", "));
            ctx.set_activity(activity).await;
        }
    }

    fn get_unit_from_opt(&self, option: &CommandDataOption) -> Option<String> {
        match &option.resolved {
            Some(CommandDataOptionValue::String(name)) => Some(name.clone()),
            _ => None,
        }
    }

    fn parse_command(&self, interaction: &ApplicationCommandInteraction) -> Option<UserCommand> {
        let (name, options) = match self.config.command_type {
            CommandType::Single => {
                let sub = interaction.data.options.get(0)?;
                (&sub.name, &sub.options)
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
        GuildId::set_application_commands(&GuildId(self.config.guild_id), &ctx.http, |builder| {
            build_commands(&self.config.units, &self.config.command_type, builder)
        })
        .await
        .unwrap();

        let mut stream = self.update_activity_stream().await.unwrap();
        self.update_activity(&ctx).await;
        while stream.next().await.is_some() {
            self.update_activity(&ctx).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(interaction) = interaction {
            match self.parse_command(&interaction) {
                Some(command) => {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        })
                        .await
                        .unwrap();
                    let response_content = match command
                        .run(self.systemd_status_manager.as_ref(), self.config.as_ref())
                        .await
                    {
                        Ok(value) => value,
                        Err(value) => value.to_string(),
                    };
                    interaction
                        .create_followup_message(&ctx.http, |response| {
                            response.content(response_content)
                        })
                        .await
                        .unwrap();
                }
                _ => {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|data| data.content("Invalid command"))
                        })
                        .await
                        .unwrap();
                }
            }
        }
    }
}
