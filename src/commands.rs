use crate::client::{BoxedError, Context, Data};
use crate::config::{Command, CommandType, UnitCollection};
use poise::command;
use poise::serenity_prelude::AutocompleteChoice;
use std::sync::Arc;

fn get_potential_units<'a>(
    command: &'a str,
    partial: &'a str,
    units: &'a UnitCollection,
) -> Vec<(&'a str, &'a str)> {
    let Ok(command) = Command::try_from(command) else {
        return Vec::new();
    };
    units[&command]
        .iter()
        .filter(move |unit| unit.starts_with(partial))
        .map(|unit| {
            let alias: &str = unit.strip_suffix(".service").unwrap_or(unit);
            (alias, unit.as_str())
        })
        .collect()
}

async fn autocomplete_units<'a>(ctx: Context<'a>, partial: &'a str) -> Vec<AutocompleteChoice> {
    get_potential_units(&ctx.command().name, partial, &ctx.data().units)
        .into_iter()
        .map(|(name, value)| AutocompleteChoice::new(name, value))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn to_names(commands: &Vec<poise::structs::Command<Arc<Data>, BoxedError>>) -> Vec<&str> {
        commands
            .into_iter()
            .map(|command| command.name.as_str())
            .collect()
    }

    #[test]
    fn test_potential_units() {
        assert_eq!(
            get_potential_units(
                "start",
                "ab",
                &HashMap::from([(
                    Command::Start,
                    vec![
                        String::from("ab.service"),
                        String::from("abc.service"),
                        String::from("acd.service")
                    ],
                )]),
            ),
            vec![("ab", "ab.service"), ("abc", "abc.service")]
        );
    }

    #[test]
    fn get_multiple_commands() {
        assert_eq!(
            to_names(&get_commands(CommandType::Multiple)),
            vec!["start", "stop", "restart", "status"]
        );
    }

    #[test]
    fn get_single_commands() {
        let commands = get_commands(CommandType::Single);
        assert_eq!(to_names(&commands), vec!["systemctl"]);
        assert_eq!(
            to_names(&commands[0].subcommands),
            vec!["start", "stop", "restart", "status"]
        );
    }
}
