use crate::client::{CommandContext, Context, Data};
use crate::config::{Command, CommandType, UnitCollection};
use anyhow::Result;
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
    get_potential_units(&ctx.command().name, partial, &ctx.get_units())
        .into_iter()
        .map(|(name, value)| AutocompleteChoice::new(name, value))
        .collect()
}

async fn start_inner(ctx: impl CommandContext, unit: String) -> Result<()> {
    ctx.defer_response().await?;
    ctx.get_units().ensure_allowed(&unit, Command::Start)?;
    ctx.get_systemctl().run(&["start", &unit]).await?;
    ctx.respond(format!("Started {}", unit)).await?;
    Ok(())
}

/// Starts units
#[command(slash_command)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "The unit to start"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<()> {
    start_inner(ctx, unit).await
}

async fn stop_inner(ctx: impl CommandContext, unit: String) -> Result<()> {
    ctx.defer_response().await?;
    ctx.get_units().ensure_allowed(&unit, Command::Stop)?;
    ctx.get_systemctl().run(&["stop", &unit]).await?;
    ctx.respond(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Stops units
#[command(slash_command)]
pub async fn stop(
    ctx: Context<'_>,
    #[description = "The unit to stop"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<()> {
    stop_inner(ctx, unit).await
}

async fn restart_inner(ctx: impl CommandContext, unit: String) -> Result<()> {
    ctx.defer_response().await?;
    ctx.get_units().ensure_allowed(&unit, Command::Restart)?;
    ctx.get_systemctl().run(&["restart", &unit]).await?;
    ctx.respond(format!("Stopped {}", unit)).await?;
    Ok(())
}

/// Restarts units
#[command(slash_command)]
pub async fn restart(
    ctx: Context<'_>,
    #[description = "The unit to restart"]
    #[autocomplete = "autocomplete_units"]
    unit: String,
) -> Result<()> {
    restart_inner(ctx, unit).await
}

async fn status_inner(ctx: impl CommandContext, unit: Option<String>) -> Result<()> {
    ctx.defer_response().await?;
    let systemd_status_manager = ctx.get_systemd_status_manager();
    let response = match unit {
        Some(unit) => {
            ctx.get_units().ensure_allowed(&unit, Command::Status)?;
            systemd_status_manager.status(&unit).await?
        }
        None => {
            let lines = systemd_status_manager
                .statuses(&ctx.get_units()[&Command::Status])
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
    ctx.respond(response).await?;
    Ok(())
}

/// Checks units' status
#[command(slash_command)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "The unit to check"]
    #[autocomplete = "autocomplete_units"]
    unit: Option<String>,
) -> Result<()> {
    status_inner(ctx, unit).await
}

pub fn get_commands(
    command_type: CommandType,
) -> Vec<poise::structs::Command<Arc<Data>, anyhow::Error>> {
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
    use crate::{client::MockCommandContext, systemctl::MockSystemctl};
    use mockall::predicate;
    use std::collections::HashMap;

    fn to_names(commands: &Vec<poise::structs::Command<Arc<Data>, anyhow::Error>>) -> Vec<&str> {
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
                &UnitCollection::from(HashMap::from([(
                    Command::Start,
                    vec![
                        String::from("ab.service"),
                        String::from("abc.service"),
                        String::from("acd.service")
                    ],
                )])),
            ),
            vec![("ab", "ab.service"), ("abc", "abc.service")]
        );
    }

    #[tokio::test]
    async fn test_start() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        ctx.expect_get_units()
            .return_const(Arc::from(UnitCollection::from(HashMap::from([(
                Command::Start,
                vec!["startable.service".to_string()],
            )]))));
        ctx.expect_get_systemctl().returning(|| {
            let mut systemctl = MockSystemctl::new();
            systemctl
                .expect_run()
                .with(predicate::eq([
                    "start".to_string(),
                    "startable.service".to_string(),
                ]))
                .returning(|_| Ok(()));
            Arc::from(systemctl)
        });
        ctx.expect_respond()
            .with(predicate::eq("Started startable.service".to_string()))
            .returning(|_| Ok(()));
        assert_eq!(
            start_inner(ctx, "startable.service".to_string()).await.ok(),
            Some(())
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
