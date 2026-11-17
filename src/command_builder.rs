use crate::{
    command::Command,
    config::{CommandType, CommandUnits},
};
use serenity::{
    all::{
        CommandOptionType, Context, CreateCommand, CreateCommandOption, EventHandler, GuildId,
        Ready,
    },
    async_trait,
};

pub fn build_commands(units: &CommandUnits, command_type: &CommandType) -> Vec<CreateCommand> {
    let command_iter = Command::VALUES
        .iter()
        .filter(|command| !&units[command].is_empty())
        .map(|command| {
            let name = command.to_string();

            let description = {
                let mut capitalized_name = name.clone();
                capitalized_name[0..1].make_ascii_uppercase();
                format!("{} units", capitalized_name)
            };

            let option = {
                let base = CreateCommandOption::new(
                    CommandOptionType::String,
                    "unit",
                    format!("The unit to {}", name),
                )
                .required(true);
                units[command].iter().fold(base, |option, unit| {
                    let alias = unit.strip_suffix(".service").unwrap_or(unit);
                    option.add_string_choice(alias, unit)
                })
            };

            (name, description, option)
        });

    match &command_type {
        CommandType::Single => {
            let options = command_iter
                .map(|(name, description, option)| {
                    CreateCommandOption::new(CommandOptionType::SubCommand, name, description)
                        .add_sub_option(option)
                })
                .collect();
            let command = CreateCommand::new("systemctl")
                .description("Controls units")
                .set_options(options);
            vec![command]
        }
        CommandType::Multiple => command_iter
            .map(|(name, description, option)| {
                CreateCommand::new(name)
                    .description(description)
                    .add_option(option)
            })
            .collect(),
    }
}

pub struct CommandBuilder {
    pub command_type: CommandType,
    pub guild_id: GuildId,
    pub units: &'static CommandUnits,
}

#[async_trait]
impl EventHandler for CommandBuilder {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        self.guild_id
            .set_commands(&ctx.http, build_commands(self.units, &self.command_type))
            .await
            .expect("Unable to set commands");
    }
}
