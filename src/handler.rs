use crate::builder::build_command;
use crate::command::UserCommand;
use crate::config::{Unit, UnitPermission};
use crate::systemctl::{statuses, SystemctlError, SystemctlManager};
use futures::future::join_all;
use futures::StreamExt;
use indexmap::IndexMap;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::{Activity, Ready};
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};
use tokio_stream::StreamMap;
use zbus::PropertyStream;

pub struct Handler<'a> {
    pub guild_id: GuildId,
    pub units: IndexMap<String, Unit>,
    systemctl: SystemctlManager<'a>,
}

impl Handler<'_> {
    pub async fn new<'a>(
        guild_id: GuildId,
        units: IndexMap<String, Unit>,
    ) -> Result<Handler<'a>, SystemctlError> {
        Ok(Handler {
            guild_id,
            units,
            systemctl: SystemctlManager::new().await?,
        })
    }

    fn units_that_allow_status_iter(&self) -> impl Iterator<Item = &Unit> {
        self.units
            .values()
            .filter(|unit| unit.permissions.contains(&UnitPermission::Status))
    }

    async fn update_activity_stream(
        &self,
    ) -> Result<StreamMap<&str, PropertyStream<'_, String>>, SystemctlError> {
        let streams = self
            .units_that_allow_status_iter()
            .map(|unit| self.systemctl.status_stream(unit.name.as_str()));
        let streams = join_all(streams)
            .await
            .into_iter()
            .collect::<Result<Vec<PropertyStream<String>>, SystemctlError>>()?;
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
        let statuses = statuses(&self.systemctl, units).await;
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

    fn get_unit_from_opt(&self, option: &ApplicationCommandInteractionDataOption) -> Option<&Unit> {
        match &option.resolved {
            Some(ApplicationCommandInteractionDataOptionValue::String(name)) => {
                self.units.get(name)
            }
            _ => None,
        }
    }

    fn parse_command(&self, interaction: &ApplicationCommandInteraction) -> Option<UserCommand> {
        let sub_command = interaction.data.options.get(0)?.to_owned();
        match sub_command.name.as_str() {
            "start" => Some(UserCommand::Start {
                unit: self.get_unit_from_opt(sub_command.options.get(0)?)?,
            }),
            "stop" => Some(UserCommand::Stop {
                unit: self.get_unit_from_opt(sub_command.options.get(0)?)?,
            }),
            "restart" => Some(UserCommand::Restart {
                unit: self.get_unit_from_opt(sub_command.options.get(0)?)?,
            }),
            "status" => {
                let option = sub_command.options.get(0);
                let units = match option {
                    Some(option) => vec![self.get_unit_from_opt(option)?],
                    None => self.units_that_allow_status_iter().collect(),
                };
                Some(UserCommand::Status { units })
            }
            _ => None,
        }
    }
}

#[async_trait]
impl EventHandler for Handler<'_> {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            builder.create_application_command(|command| build_command(&self.units, command))
        })
        .await
        .unwrap();

        let mut stream = self.update_activity_stream().await.unwrap();
        self.update_activity(&ctx).await;
        while let Some(_) = stream.next().await {
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
                    let response_content = match command.run(&self.systemctl).await {
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
