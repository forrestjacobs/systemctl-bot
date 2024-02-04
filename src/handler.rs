use crate::builder::build_commands;
use crate::command::UserCommand;
use crate::config::CommandType;
use crate::parser::parse_command;
use crate::status_monitor::monitor_status;
use crate::systemd_status::SystemdStatusManager;
use crate::units::Units;
use serenity::all::CommandInteraction;
use serenity::async_trait;
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseFollowup, CreateInteractionResponseMessage,
};
use serenity::client::{Context, EventHandler};
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;

pub struct Handler {
    pub guild_id: GuildId,
    pub command_type: CommandType,
    pub units: Units,
    systemd_status_manager: SystemdStatusManager,
}

impl Handler {
    pub async fn new(
        guild_id: GuildId,
        command_type: CommandType,
        units: Units,
    ) -> Result<Handler, zbus::Error> {
        Ok(Handler {
            guild_id,
            command_type,
            units,
            systemd_status_manager: SystemdStatusManager::new().await?,
        })
    }

    async fn handle_command(
        &self,
        command: UserCommand,
        ctx: Context,
        interaction: CommandInteraction,
    ) {
        interaction.defer(&ctx).await.unwrap();
        let run = command.run(&self.units, &self.systemd_status_manager).await;
        let content = match run {
            Ok(value) => value,
            Err(value) => value.to_string(),
        };
        interaction
            .create_followup(
                &ctx,
                CreateInteractionResponseFollowup::new().content(content),
            )
            .await
            .unwrap();
    }
}

async fn handle_invalid_command(ctx: Context, interaction: CommandInteraction) {
    interaction
        .create_response(
            &ctx,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Invalid command"),
            ),
        )
        .await
        .unwrap();
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        let commands = build_commands(&self.units, &self.command_type);
        self.guild_id.set_commands(&ctx, commands).await.unwrap();

        let _ = monitor_status(&self.units, &ctx, &self.systemd_status_manager).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(interaction) = interaction {
            match parse_command(&self.command_type, &interaction) {
                Some(command) => self.handle_command(command, ctx, interaction).await,
                _ => handle_invalid_command(ctx, interaction).await,
            }
        }
    }
}
