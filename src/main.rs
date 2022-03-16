mod config;

use indexmap::IndexMap;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::interactions::application_command::ApplicationCommandOptionType;
use serenity::model::interactions::{Interaction, InteractionResponseType};

use config::{get_config, Service};

#[derive(Debug)]
enum CommandType {
    Start,
    Stop,
}

struct Handler {
    guild_id: GuildId,
    services: IndexMap<String, Service>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            builder.create_application_command(|command| {
                command.name("systemctl").description("Controls services");
                command.create_option(|start| {
                    start
                        .name("start")
                        .description("Starts services")
                        .kind(ApplicationCommandOptionType::SubCommand);
                    start.create_sub_option(|service_opt| {
                        service_opt
                            .name("service")
                            .description("The service to start")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true);
                        for name in self.services.keys() {
                            service_opt.add_string_choice(name, name);
                        }
                        service_opt
                    })
                });
                command.create_option(|stop| {
                    stop.name("stop")
                        .description("Stops services")
                        .kind(ApplicationCommandOptionType::SubCommand);
                    stop.create_sub_option(|service_opt| {
                        service_opt
                            .name("service")
                            .description("The service to stop")
                            .kind(ApplicationCommandOptionType::String)
                            .required(true);
                        for name in self.services.keys() {
                            service_opt.add_string_choice(name, name);
                        }
                        service_opt
                    })
                });
                command
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
                .and_then(|opt| self.services.get(&opt.name));

            match (kind, service) {
                (Some(kind), Some(service)) => {
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::DeferredChannelMessageWithSource)
                        })
                        .await
                        .unwrap();
                    // TODO actually do this
                    println!("systemctl {:?} {}", kind, service.unit);
                    interaction
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|data| data.content("Done!"))
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

#[tokio::main]
async fn main() {
    let config = get_config();

    let mut client = Client::builder(config.discord_token)
        .event_handler(Handler {
            guild_id: GuildId(config.guild_id),
            services: config.services,
        })
        .application_id(config.application_id)
        .await
        .unwrap();

    client.start().await.unwrap();
}
