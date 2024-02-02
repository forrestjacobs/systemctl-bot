use crate::builder::build_commands;
use crate::command::UserCommand;
use crate::config::CommandType;
use crate::parser::parse_command;
use crate::status_monitor::monitor_status;
use crate::systemd_status::SystemdStatusManager;
use crate::units::Unit;
use indexmap::IndexMap;
use serenity::async_trait;
use serenity::client::{Context, EventHandler};
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::Interaction;
use serenity::model::application::interaction::InteractionResponseType::{
    ChannelMessageWithSource, DeferredChannelMessageWithSource,
};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;

pub struct Handler {
    pub guild_id: GuildId,
    pub command_type: CommandType,
    pub units: IndexMap<String, Unit>,
    systemd_status_manager: SystemdStatusManager,
}

impl Handler {
    pub async fn new(
        guild_id: GuildId,
        command_type: CommandType,
        units: IndexMap<String, Unit>,
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
        interaction: ApplicationCommandInteraction,
    ) {
        interaction
            .create_interaction_response(&ctx, |r| r.kind(DeferredChannelMessageWithSource))
            .await
            .unwrap();
        let run = command.run(&self.units, &self.systemd_status_manager).await;
        let content = match run {
            Ok(value) => value,
            Err(value) => value.to_string(),
        };
        interaction
            .create_followup_message(&ctx, |r| r.content(content))
            .await
            .unwrap();
    }

    async fn handle_invalid_command(
        &self,
        ctx: Context,
        interaction: ApplicationCommandInteraction,
    ) {
        interaction
            .create_interaction_response(&ctx, |r| {
                r.kind(ChannelMessageWithSource)
                    .interaction_response_data(|data| data.content("Invalid command"))
            })
            .await
            .unwrap();
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        GuildId::set_application_commands(&self.guild_id, &ctx.http, |builder| {
            build_commands(&self.units, &self.command_type, builder)
        })
        .await
        .unwrap();

        let _ = monitor_status(&self.units, &ctx, &self.systemd_status_manager).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(interaction) = interaction {
            match parse_command(&self.command_type, &interaction) {
                Some(command) => self.handle_command(command, ctx, interaction).await,
                _ => self.handle_invalid_command(ctx, interaction).await,
            }
        }
    }
}
