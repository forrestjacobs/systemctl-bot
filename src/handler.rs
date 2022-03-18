use indexmap::IndexMap;

use serenity::async_trait;
use serenity::builder::CreateApplicationCommandOption;
use serenity::client::{Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteractionDataOptionValue, ApplicationCommandOptionType,
};
use serenity::model::interactions::{Interaction, InteractionResponseType};

use std::process::Command;

use crate::config::Service;

#[derive(Debug)]
enum CommandType {
    Start,
    Stop,
}

impl CommandType {
    fn as_systemctl_arg(&self) -> &str {
        match self {
            CommandType::Start => "start",
            CommandType::Stop => "stop",
        }
    }
}

pub struct Handler {
    pub guild_id: GuildId,
    pub services: IndexMap<String, Service>,
}

fn setup_service_option<'a, I: Iterator<Item = &'a String>>(
    command: &mut CreateApplicationCommandOption,
    services: I,
) -> &mut CreateApplicationCommandOption {
    command
        .name("service")
        .kind(ApplicationCommandOptionType::String);
    for service in services {
        command.add_string_choice(service, service);
    }
    command
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            builder.create_application_command(|command| {
                command
                    .name("systemctl")
                    .description("Controls services")
                    .create_option(|start| {
                        start
                            .name("start")
                            .description("Starts services")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|opt| {
                                setup_service_option(opt, self.services.keys())
                                    .description("The service to start")
                                    .required(true)
                            })
                    })
                    .create_option(|stop| {
                        stop.name("stop")
                            .description("Stops services")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|opt| {
                                setup_service_option(opt, self.services.keys())
                                    .description("The service to stop")
                                    .required(true)
                            })
                    })
            })
        })
        .await
        .unwrap();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(interaction) = interaction {
            let sub_command = interaction.data.options.get(0);
            let kind = sub_command.and_then(|sub_command| match sub_command.name.as_str() {
                "start" => Some(CommandType::Start),
                "stop" => Some(CommandType::Stop),
                _ => None,
            });
            let service = sub_command
                .and_then(|sub_command| sub_command.options.get(0))
                .and_then(|option| match &option.resolved {
                    Some(ApplicationCommandInteractionDataOptionValue::String(value)) => {
                        self.services.get(value)
                    }
                    _ => None,
                });

            match (kind, service) {
                (Some(kind), Some(service)) => {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        })
                        .await
                        .unwrap();
                    let command_result = Command::new("systemctl")
                        .arg(kind.as_systemctl_arg())
                        .arg(&service.unit)
                        .output();
                    let response_content = match command_result {
                        Ok(output) if output.status.success() => String::from_utf8(output.stdout).unwrap(),
                        Ok(output) => format!("Error: {}", String::from_utf8(output.stderr).unwrap()),
                        Err(e) => format!("Error: {}", e),
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
