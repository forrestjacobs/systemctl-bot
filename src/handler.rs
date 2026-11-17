use crate::command::CommandRunner;
use crate::parser::Parser;
use async_trait::async_trait;
use serenity::all::{
    Context, CreateInteractionResponse, CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage, EventHandler, Interaction,
};

pub struct Handler {
    pub parser: Parser,
    pub command_runner: CommandRunner,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(interaction) = interaction {
            match self.parser.parse(&interaction) {
                Some((command, unit)) => {
                    interaction
                        .create_response(
                            &ctx.http,
                            CreateInteractionResponse::Defer(
                                CreateInteractionResponseMessage::new(),
                            ),
                        )
                        .await
                        .unwrap();
                    let response_content = match self.command_runner.run(&command, &unit).await {
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
