use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use serenity::all::{
    CacheHttp, CommandDataOptionValue, Context, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, EventHandler, Http,
    Interaction,
};

use crate::{
    command::Command,
    config::{CommandType, SystemctlBotConfig},
    systemd::Systemd,
};

pub struct CommandHandler {
    pub config: Arc<SystemctlBotConfig>,
    pub systemd: Arc<Systemd>,
}

impl CommandHandler {
    async fn handle_interaction(&self, http: &Http, interaction: Interaction) {
        let Interaction::Command(interaction) = interaction else {
            eprintln!("Unexpected interaction type {:?}", interaction.kind());
            return;
        };

        let command_and_unit: Result<(Command, &str), String> = (|| {
            let (name, options) = match self.config.command_type {
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
            if !self.config.units.command_units[&command].contains(unit) {
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
                eprintln!("{}", e);
                let message = CreateInteractionResponseMessage::new().content("Invalid command");
                let message = CreateInteractionResponse::Message(message);
                if let Err(error) = interaction.create_response(http, message).await {
                    eprintln!("Error responding: {}", error);
                }
                return;
            }
        };

        let result: serenity::Result<()> = async {
            let defer = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
            interaction.create_response(http, defer).await?;

            let content = match self.systemd.run(&command, unit).await {
                Ok(_) => {
                    let verb = match command {
                        Command::Start => "Started",
                        Command::Stop => "Stopped",
                        Command::Restart => "Restarted",
                    };
                    format!("{} {}", verb, unit)
                }
                Err(error) => error.to_string(),
            };

            let followup = CreateInteractionResponseFollowup::new().content(content);
            interaction.create_followup(http, followup).await?;
            Ok(())
        }
        .await;
        if let Err(error) = result {
            eprint!("Error handling interaction: {}", error);
        }
    }
}

#[async_trait]
impl EventHandler for CommandHandler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.handle_interaction(ctx.http(), interaction).await
    }
}
