use crate::command::UserCommand;
use crate::config::{Service, ServicePermission};
use indexmap::IndexMap;
use serenity::async_trait;
use serenity::builder::CreateApplicationCommandOption;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOption,
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};

pub struct Handler {
    pub guild_id: GuildId,
    pub services: IndexMap<String, Service>,
}

fn setup_service_option<'a>(
    command: &'a mut CreateApplicationCommandOption,
    services: Vec<&String>,
) -> &'a mut CreateApplicationCommandOption {
    command
        .name("service")
        .kind(ApplicationCommandOptionType::String);
    for service in services {
        command.add_string_choice(service, service);
    }
    command
}

impl Handler {
    fn with_service_names<P: Fn(&Service) -> bool, F: FnOnce(Vec<&String>)>(
        &self,
        predicate: P,
        f: F,
    ) {
        if self.services.values().any(&predicate) {
            let services = self
                .services
                .iter()
                .filter(|(_, service)| predicate(service))
                .map(|(name, _)| name)
                .collect::<Vec<&String>>();
            f(services);
        }
    }

    fn get_service_from_opt(
        &self,
        option: &ApplicationCommandInteractionDataOption,
    ) -> Option<&Service> {
        match &option.resolved {
            Some(ApplicationCommandInteractionDataOptionValue::String(name)) => {
                self.services.get(name)
            }
            _ => None,
        }
    }

    fn parse_command(&self, interaction: &ApplicationCommandInteraction) -> Option<UserCommand> {
        let sub_command = interaction.data.options.get(0)?.to_owned();
        match sub_command.name.as_str() {
            "start" => Some(UserCommand::Start {
                service: self.get_service_from_opt(sub_command.options.get(0)?)?,
            }),
            "stop" => Some(UserCommand::Stop {
                service: self.get_service_from_opt(sub_command.options.get(0)?)?,
            }),
            "restart" => Some(UserCommand::Restart {
                service: self.get_service_from_opt(sub_command.options.get(0)?)?,
            }),
            "status" => {
                let option = sub_command.options.get(0);
                let services = match option {
                    Some(option) => vec![self.get_service_from_opt(option)?],
                    None => self
                        .services
                        .values()
                        .filter(|s| s.permissions.contains(&ServicePermission::Status))
                        .collect(),
                };
                Some(UserCommand::Status { services })
            }
            _ => None,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            builder.create_application_command(|command| {
                command.name("systemctl").description("Controls services");
                self.with_service_names(
                    |service| service.permissions.contains(&ServicePermission::Start),
                    |services| {
                        command.create_option(|sub| {
                            sub.name("start")
                                .description("Starts services")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|opt| {
                                    setup_service_option(opt, services)
                                        .description("The service to start")
                                        .required(true)
                                })
                        });
                    },
                );
                self.with_service_names(
                    |service| service.permissions.contains(&ServicePermission::Stop),
                    |services| {
                        command.create_option(|sub| {
                            sub.name("stop")
                                .description("Stops services")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|opt| {
                                    setup_service_option(opt, services)
                                        .description("The service to stop")
                                        .required(true)
                                })
                        });
                    },
                );
                self.with_service_names(
                    |service| {
                        service.permissions.contains(&ServicePermission::Stop)
                            && service.permissions.contains(&ServicePermission::Start)
                    },
                    |services| {
                        command.create_option(|sub| {
                            sub.name("restart")
                                .description("Restarts services")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|opt| {
                                    setup_service_option(opt, services)
                                        .description("The service to restart")
                                        .required(true)
                                })
                        });
                    },
                );
                self.with_service_names(
                    |service| service.permissions.contains(&ServicePermission::Status),
                    |services| {
                        command.create_option(|sub| {
                            sub.name("status")
                                .description("Checks services' status")
                                .kind(ApplicationCommandOptionType::SubCommand)
                                .create_sub_option(|opt| {
                                    setup_service_option(opt, services)
                                        .description("The service to check")
                                })
                        });
                    },
                );
                command
            })
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
