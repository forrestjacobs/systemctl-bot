use crate::builder::build_command;
use crate::command::UserCommand;
use crate::config::{Unit, UnitPermission};
use indexmap::IndexMap;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};

pub struct Handler {
    pub guild_id: GuildId,
    pub units: IndexMap<String, Unit>,
}

impl Handler {
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
                    None => self
                        .units
                        .values()
                        .filter(|unit| unit.permissions.contains(&UnitPermission::Status))
                        .collect(),
                };
                Some(UserCommand::Status { units })
            }
            _ => None,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            builder.create_application_command(|command| build_command(&self.units, command))
        })
        .await
        .unwrap();
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
                    let response_content = match command.run().await {
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
