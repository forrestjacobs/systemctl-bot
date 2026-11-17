use std::str::FromStr;

use mockall::automock;
use serenity::all::{
    CommandDataOptionValue, CommandInteraction, Context, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EventHandler, Http,
    Interaction, Message,
};
use serenity::async_trait;

use crate::{
    command::Command,
    config::{CommandType, CommandUnits},
    systemd::Systemd,
};

#[async_trait]
#[automock]
trait ResponseHandler {
    async fn create_response(
        &self,
        interaction: &CommandInteraction,
        builder: CreateInteractionResponse,
    ) -> serenity::Result<()>;
    async fn create_followup(
        &self,
        interaction: &CommandInteraction,
        builder: CreateInteractionResponseFollowup,
    ) -> serenity::Result<Message>;
}

struct ResponseHandlerImpl<'a> {
    http: &'a Http,
}

#[async_trait]
impl ResponseHandler for ResponseHandlerImpl<'_> {
    async fn create_response(
        &self,
        interaction: &CommandInteraction,
        builder: CreateInteractionResponse,
    ) -> serenity::Result<()> {
        interaction.create_response(&self.http, builder).await
    }
    async fn create_followup(
        &self,
        interaction: &CommandInteraction,
        builder: CreateInteractionResponseFollowup,
    ) -> serenity::Result<Message> {
        interaction.create_followup(&self.http, builder).await
    }
}

pub struct CommandHandler<'a, S: Systemd> {
    pub command_type: CommandType,
    pub systemd: &'a S,
    pub units: &'a CommandUnits,
}

impl<S: Systemd> CommandHandler<'_, S> {
    async fn handle_interaction<R: ResponseHandler>(
        &self,
        responder: &R,
        interaction: Interaction,
    ) {
        let Interaction::Command(interaction) = interaction else {
            eprintln!("Unexpected interaction type {:?}", interaction.kind());
            return;
        };

        let command_and_unit: Result<(Command, &str), String> = (|| {
            let (name, options) = match self.command_type {
                CommandType::Single => {
                    let sub = interaction
                        .data
                        .options
                        .first()
                        .ok_or("Unable to get subcommand options".to_string())?;
                    let CommandDataOptionValue::SubCommand(opts) = &sub.value else {
                        return Err("Unable to get subcommand".into());
                    };
                    (&sub.name, opts)
                }
                CommandType::Multiple => (&interaction.data.name, &interaction.data.options),
            };
            let command = Command::from_str(name).map_err(|e| e.to_string())?;
            let CommandDataOptionValue::String(unit) =
                &options.first().ok_or("Unable to get option")?.value
            else {
                return Err("Unable to get unit".into());
            };
            if !self.units[&command].contains(unit) {
                return Err(format!(
                    "User {} attempted to call {} {}",
                    interaction.user.name, command, unit
                ));
            }
            Ok((command, unit.as_str()))
        })();

        let (command, unit) = match command_and_unit {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{e}");
                let message = CreateInteractionResponseMessage::new().content("Invalid command");
                let message = CreateInteractionResponse::Message(message);
                if let Err(error) = responder.create_response(&interaction, message).await {
                    eprintln!("Error responding: {error}");
                }
                return;
            }
        };

        let result: serenity::Result<()> = async {
            let defer = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
            responder.create_response(&interaction, defer).await?;

            let content = match self.systemd.run(&command, unit).await {
                Ok(()) => {
                    let verb = match command {
                        Command::Start => "Started",
                        Command::Stop => "Stopped",
                        Command::Restart => "Restarted",
                    };
                    format!("{verb} {unit}")
                }
                Err(error) => error.to_string(),
            };

            let followup = CreateInteractionResponseFollowup::new().content(content);
            responder.create_followup(&interaction, followup).await?;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            eprint!("Error handling interaction: {error}");
        }
    }
}

#[async_trait]
impl<S: Systemd + Sync> EventHandler for CommandHandler<'_, S> {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let handler = ResponseHandlerImpl {
            http: ctx.http.as_ref(),
        };
        self.handle_interaction(&handler, interaction).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mockall::predicate::{eq, function};
    use serenity::{
        all::{CommandOptionType, CommandType as SerenityCommandType, InteractionType},
        json::{self, from_value, json, to_value},
    };

    use crate::systemd::MockSystemd;

    use super::*;

    fn units_fixture() -> CommandUnits {
        HashMap::from([
            (
                Command::Start,
                vec![
                    "startable.service".to_string(),
                    "restartable.service".to_string(),
                ],
            ),
            (
                Command::Stop,
                vec![
                    "stoppable.service".to_string(),
                    "restartable.service".to_string(),
                ],
            ),
            (Command::Restart, vec!["restartable.service".to_string()]),
        ])
    }

    #[tokio::test]
    async fn handle_invalid_interaction() {
        let systemctl = MockSystemd::new();
        let responder = MockResponseHandler::new();

        CommandHandler {
            command_type: CommandType::Single,
            systemd: &systemctl,
            units: &units_fixture(),
        }
        .handle_interaction(
            &responder,
            from_value(json!({
                "type": InteractionType::Ping,
                "id": 1,
                "application_id": 1,
                "token": "",
                "version": 1,
            }))
            .unwrap(),
        )
        .await;
    }

    fn make_command_interaction(data: &json::Value) -> Interaction {
        from_value(json!({
            "type": InteractionType::Command,
            "id": 1,
            "application_id": 1,
            "data": data,
            "channel_id": 1,
            "token": "",
            "version": 1,
            "locale": "en-US",
            "entitlements": [],
            "attachment_size_limit": 0,
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn make_invalid_command() {
        let systemctl = MockSystemd::new();
        let mut handler = CommandHandler {
            command_type: CommandType::Single,
            systemd: &systemctl,
            units: &units_fixture(),
        };

        let mut responder = MockResponseHandler::new();
        responder
            .expect_create_response()
            .with(
                function(|i: &CommandInteraction| i.id.get() == 1),
                function(|m| {
                    to_value(m).unwrap()["data"]["content"].as_str() == Some("Invalid command")
                }),
            )
            .returning(|_, _| Box::pin(async { Ok(()) }));

        handler
            .handle_interaction(
                &responder,
                make_command_interaction(&json!({
                    "type": SerenityCommandType::ChatInput,
                    "id": 1,
                    "name": "systemctl",
                    // No options
                })),
            )
            .await;

        handler
            .handle_interaction(
                &responder,
                make_command_interaction(&json!({
                    "type": SerenityCommandType::ChatInput,
                    "id": 1,
                    "name": "systemctl",
                    "options": [{
                        "type": CommandOptionType::String,
                        "name": "start",
                        // Bad option type
                        "value": "start",
                    }],
                })),
            )
            .await;

        handler.command_type = CommandType::Multiple;
        handler
            .handle_interaction(
                &responder,
                make_command_interaction(&json!({
                    "type": SerenityCommandType::ChatInput,
                    "id": 1,
                    "name": "start",
                    "options": [{
                        // Bad option type
                        "type": CommandOptionType::Integer,
                        "name": "unit",
                        "value": 1,
                    }],
                })),
            )
            .await;
        handler
            .handle_interaction(
                &responder,
                make_command_interaction(&json!({
                    "type": SerenityCommandType::ChatInput,
                    "id": 1,
                    // Bad command name
                    "name": "recombobulate",
                    "options": [{
                        "type": CommandOptionType::String,
                        "name": "unit",
                        "value": "startable.service",
                    }],
                })),
            )
            .await;
        handler
            .handle_interaction(
                &responder,
                make_command_interaction(&json!({
                    "type": SerenityCommandType::ChatInput,
                    "id": 1,
                    "name": "start",
                    "options": [{
                        "type": CommandOptionType::String,
                        "name": "unit",
                        "value": "stoppable.service",
                    }],
                })),
            )
            .await;
    }

    #[tokio::test]
    async fn handle_interaction() {
        let mut systemctl = MockSystemd::new();
        systemctl
            .expect_run()
            .with(eq(Command::Start), eq("startable.service"))
            .returning(|_, _| Box::pin(async { Ok(()) }));

        let mut responder = MockResponseHandler::new();
        responder
            .expect_create_response()
            .with(
                function(|i: &CommandInteraction| i.id.get() == 1),
                function(|m| to_value(m).unwrap()["type"].as_i64() == Some(5)),
            )
            .returning(|_, _| Box::pin(async { Ok(()) }));
        responder
            .expect_create_followup()
            .with(
                function(|i: &CommandInteraction| i.id.get() == 1),
                function(|m| {
                    to_value(m).unwrap()["content"].as_str() == Some("Started startable.service")
                }),
            )
            .returning(|_, _| Box::pin(async { Err(serenity::Error::Other("test")) }));

        CommandHandler {
            command_type: CommandType::Single,
            systemd: &systemctl,
            units: &units_fixture(),
        }
        .handle_interaction(
            &responder,
            make_command_interaction(&json!({
                "type": SerenityCommandType::ChatInput,
                "id": 1,
                "name": "systemctl",
                "options": [{
                    "type": CommandOptionType::SubCommand,
                    "name": "start",
                    "options": [{
                        "type": CommandOptionType::String,
                        "name": "unit",
                        "value": "startable.service",
                    }],
                }],
            })),
        )
        .await;
    }
}
