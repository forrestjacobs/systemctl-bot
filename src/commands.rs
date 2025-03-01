use crate::client::{BoxedError, Context, Data};
use crate::config::{Command, CommandType};
use poise::command;
use poise::serenity_prelude::AutocompleteChoice;
use std::iter::empty;
use std::sync::Arc;

async fn autocomplete_units<'a>(ctx: Context<'a>, partial: &'a str) -> Vec<AutocompleteChoice> {
    let Ok(command) = Command::try_from(ctx.command().name.as_str()) else {
        return empty().collect();
    };
    ctx.data().units[&command]
        .iter()
        .filter(move |unit| unit.starts_with(partial))
        .map(|unit| {
            let alias = unit.strip_suffix(".service").unwrap_or(unit);
            AutocompleteChoice::new(alias, unit.as_str())
        })
        .collect()
}

fn systemctl() -> tokio::process::Command {
    tokio::process::Command::new("systemctl")
}

/// Starts units
#[command(slash_command)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "The unit to start"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), BoxedError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Start)?;
    data.runner.run(systemctl().arg("start").arg(&unit)).await?;
    ctx.say(format!("Started {}", unit)).await?;
    Ok(())
}

/// Stops units
#[command(slash_command)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "The unit to stop"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), BoxedError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Stop)?;
    data.runner.run(systemctl().arg("stop").arg(&unit)).await?;
    ctx.say(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Restarts units
#[command(slash_command)]
pub async fn restart(
    ctx: Context<'_>,
    #[description = "The unit to restart"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<(), BoxedError> {
    ctx.defer().await?;
    let data = ctx.data();
    data.ensure_allowed(&unit, Command::Restart)?;
    data.runner
        .run(systemctl().arg("restart").arg(&unit))
        .await?;
    ctx.say(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Checks units' status
#[command(slash_command)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "The unit to check"]
    #[autocomplete = "autocomplete_units"]
    unit: Option<String>,
) -> Result<(), BoxedError> {
    ctx.defer().await?;
    let data = ctx.data();
    let response = match unit {
        Some(unit) => {
            data.ensure_allowed(&unit, Command::Status)?;
            data.systemd_status_manager.status(&unit).await?
        }
        None => {
            let lines = data
                .systemd_status_manager
                .statuses(&data.units[&Command::Status])
                .await
                .map(|(unit, status)| (unit, status.unwrap_or_else(|err| format!("{}", err))))
                .filter(|(_, status)| status != "inactive")
                .map(|(unit, status)| format!("{}: {}", unit, status))
                .collect::<Vec<String>>();
            if lines.is_empty() {
                String::from("Nothing is active")
            } else {
                lines.join("\n")
            }
        }
    };
    ctx.say(response).await?;
    Ok(())
}

pub fn get_commands(
    command_type: CommandType,
) -> Vec<poise::structs::Command<Arc<Data>, BoxedError>> {
    let commands = vec![start(), stop(), restart(), status()];
    match command_type {
        CommandType::Multiple => commands,
        CommandType::Single => vec![poise::Command {
            name: "systemctl".into(),
            subcommands: commands,
            ..Default::default()
        }],
    }
}
