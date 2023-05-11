use crate::builder::build_commands;
use crate::command::UserCommand;
use crate::config::{CommandType, Unit, UnitPermission};
use crate::systemd_status::SystemdStatusManager;
use futures::future::join_all;
use futures::StreamExt;
use indexmap::IndexMap;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::application::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption, CommandDataOptionValue,
};
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::{Activity, Ready};
use serenity::model::id::GuildId;
use tokio_stream::StreamMap;
use zbus::PropertyStream;

pub struct Handler {
    pub guild_id: GuildId,
    pub command_type: CommandType,
    pub units: IndexMap<String, Unit>,
    systemd_status_manager: SystemdStatusManager,
}

impl Handler {
    pub async fn new(
        guild_id: GuildId,
        command_type: CommandType,
        units: IndexMap<String, Unit>,
    ) -> Result<Handler, zbus::Error> {
        Ok(Handler {
            guild_id,
            command_type,
            units,
            systemd_status_manager: SystemdStatusManager::new().await?,
        })
    }

    fn units_that_allow_status_iter(&self) -> impl Iterator<Item = &Unit> {
        self.units
            .values()
            .filter(|unit| unit.permissions.contains(&UnitPermission::Status))
    }

    async fn update_activity_stream(
        &self,
    ) -> Result<StreamMap<&str, PropertyStream<'_, String>>, zbus::Error> {
        let streams = self.units_that_allow_status_iter().map(|unit| {
            self.systemd_status_manager
                .status_stream(unit.name.as_str())
        });
        let streams = join_all(streams)
            .await
            .into_iter()
            .collect::<Result<Vec<PropertyStream<String>>, zbus::Error>>()?;
        Ok(self
            .units_that_allow_status_iter()
            .map(|unit| unit.name.as_str())
            .zip(streams)
            .collect::<StreamMap<&str, PropertyStream<String>>>())
    }

    async fn update_activity(&self, ctx: &Context) {
        let units = self
            .units_that_allow_status_iter()
            .map(|unit| unit.name.as_str());
        let statuses = self.systemd_status_manager.statuses(units).await;
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

    fn get_unit_from_opt(&self, option: &CommandDataOption) -> Option<&Unit> {
        match &option.resolved {
            Some(CommandDataOptionValue::String(name)) => self.units.get(name),
            _ => None,
        }
    }

    fn parse_command(&self, interaction: &ApplicationCommandInteraction) -> Option<UserCommand> {
        let (name, options) = match self.command_type {
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
                    units: self.units_that_allow_status_iter().collect(),
                },
            }),
            _ => None,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            build_commands(&self.units, &self.command_type, builder)
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
                    let response_content = match command.run(&self.systemd_status_manager).await {
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
