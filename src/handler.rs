use crate::builder::build_commands;
use crate::command::CommandRunner;
use crate::config::Config;
use crate::parser::Parser;
use crate::status_monitor::StatusMonitor;
use async_trait::async_trait;
use serenity::all::{
    Context, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, GuildId, Interaction,
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
    parser: Arc<dyn Parser>,
    #[shaku(inject)]
    command_runner: Arc<dyn CommandRunner>,
    #[shaku(inject)]
    status_monitor: Arc<dyn StatusMonitor>,
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
            match self.parser.parse(&interaction) {
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
