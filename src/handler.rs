use crate::builder::build_commands;
use crate::command::CommandRunner;
use crate::config::SystemctlBotConfig;
use crate::parser::Parser;
use async_trait::async_trait;
use serenity::all::{
    Context, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, EventHandler, Interaction, Ready,
};
use std::sync::Arc;

pub struct Handler {
    pub config: Arc<SystemctlBotConfig>,
    pub parser: Parser,
    pub command_runner: CommandRunner,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _ready: Ready) {
        self.config
            .guild_id
            .set_commands(
                &ctx.http,
                build_commands(&self.config.units, &self.config.command_type),
            )
            .await
            .unwrap();
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
