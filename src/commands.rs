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
    ctx.respond(format!("Restarted {}", unit)).await?;
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
    use anyhow::bail;
    use mockall::predicate;
    use std::collections::HashMap;

    fn mock_units(ctx: &mut MockCommandContext, command: Command, units: &[&str]) {
        ctx.expect_get_units()
            .return_const(Arc::from(UnitCollection::from(HashMap::from([(
                command,
                units.into_iter().map(|unit| unit.to_string()).collect(),
            )]))));
    }

    fn disallow_systemctl_run(ctx: &mut MockCommandContext) {
        ctx.expect_get_systemctl().return_once(|| {
            let mut systemctl = MockSystemctl::new();
            systemctl.expect_run().never();
            Arc::from(systemctl)
        });
    }

    fn mock_systemctl_run(ctx: &mut MockCommandContext, args: &[&str], is_ok: bool) {
        let args: Vec<String> = args.into_iter().map(|arg| arg.to_string()).collect();
        ctx.expect_get_systemctl().return_once(move || {
            let mut systemctl = MockSystemctl::new();
            systemctl
                .expect_run()
                .with(predicate::eq(args))
                .returning(move |_| if is_ok { Ok(()) } else { bail!("Run error") });
            Arc::from(systemctl)
        });
    }

    fn mock_respond(ctx: &mut MockCommandContext, response: &str, is_ok: bool) {
        let response = response.to_string();
        ctx.expect_respond()
            .with(predicate::eq(response))
            .returning(move |_| {
                if is_ok {
                    Ok(())
                } else {
                    bail!("Response error")
                }
            });
    }

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
    async fn start_fails_on_defer() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response()
            .returning(|| bail!("Defer error"));
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            start_inner(ctx, "startable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Defer error".to_string())
        );
    }

    #[tokio::test]
    async fn start_missing_permission() {
        let mut ctx: MockCommandContext = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Start, &[]);
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            start_inner(ctx, "startable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Command is not allowed".to_string())
        );
    }

    #[tokio::test]
    async fn start_fails_on_run() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Start, &["startable.service"]);
        mock_systemctl_run(&mut ctx, &["start", "startable.service"], false);
        assert_eq!(
            start_inner(ctx, "startable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Run error".to_string())
        );
    }

    #[tokio::test]
    async fn start_fails_on_reply() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Start, &["startable.service"]);
        mock_systemctl_run(&mut ctx, &["start", "startable.service"], true);
        mock_respond(&mut ctx, "Started startable.service", false);
        assert_eq!(
            start_inner(ctx, "startable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Response error".to_string())
        );
    }

    #[tokio::test]
    async fn start() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Start, &["startable.service"]);
        mock_systemctl_run(&mut ctx, &["start", "startable.service"], true);
        mock_respond(&mut ctx, "Started startable.service", true);
        assert_eq!(
            start_inner(ctx, "startable.service".to_string()).await.ok(),
            Some(())
        );
    }

    #[tokio::test]
    async fn stop_fails_on_defer() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response()
            .returning(|| bail!("Defer error"));
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            stop_inner(ctx, "stoppable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Defer error".to_string())
        );
    }

    #[tokio::test]
    async fn stop_missing_permission() {
        let mut ctx: MockCommandContext = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Stop, &[]);
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            stop_inner(ctx, "stoppable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Command is not allowed".to_string())
        );
    }

    #[tokio::test]
    async fn stop_fails_on_run() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Stop, &["stoppable.service"]);
        mock_systemctl_run(&mut ctx, &["stop", "stoppable.service"], false);
        assert_eq!(
            stop_inner(ctx, "stoppable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Run error".to_string())
        );
    }

    #[tokio::test]
    async fn stop_fails_on_reply() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Stop, &["stoppable.service"]);
        mock_systemctl_run(&mut ctx, &["stop", "stoppable.service"], true);
        mock_respond(&mut ctx, "Stopped stoppable.service", false);
        assert_eq!(
            stop_inner(ctx, "stoppable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Response error".to_string())
        );
    }

    #[tokio::test]
    async fn stop() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Stop, &["stoppable.service"]);
        mock_systemctl_run(&mut ctx, &["stop", "stoppable.service"], true);
        mock_respond(&mut ctx, "Stopped stoppable.service", true);
        assert_eq!(
            stop_inner(ctx, "stoppable.service".to_string()).await.ok(),
            Some(())
        );
    }

    #[tokio::test]
    async fn restart_fails_on_defer() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response()
            .returning(|| bail!("Defer error"));
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            restart_inner(ctx, "restartable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Defer error".to_string())
        );
    }

    #[tokio::test]
    async fn restart_missing_permission() {
        let mut ctx: MockCommandContext = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Restart, &[]);
        disallow_systemctl_run(&mut ctx);
        assert_eq!(
            restart_inner(ctx, "restartable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Command is not allowed".to_string())
        );
    }

    #[tokio::test]
    async fn restart_fails_on_run() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Restart, &["restartable.service"]);
        mock_systemctl_run(&mut ctx, &["restart", "restartable.service"], false);
        assert_eq!(
            restart_inner(ctx, "restartable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Run error".to_string())
        );
    }

    #[tokio::test]
    async fn restart_fails_on_reply() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Restart, &["restartable.service"]);
        mock_systemctl_run(&mut ctx, &["restart", "restartable.service"], true);
        mock_respond(&mut ctx, "Restarted restartable.service", false);
        assert_eq!(
            restart_inner(ctx, "restartable.service".to_string())
                .await
                .map_err(|e| e.to_string()),
            Err("Response error".to_string())
        );
    }

    #[tokio::test]
    async fn restart() {
        let mut ctx = MockCommandContext::new();
        ctx.expect_defer_response().returning(|| Ok(()));
        mock_units(&mut ctx, Command::Restart, &["restartable.service"]);
        mock_systemctl_run(&mut ctx, &["restart", "restartable.service"], true);
        mock_respond(&mut ctx, "Restarted restartable.service", true);
        assert_eq!(
            restart_inner(ctx, "restartable.service".to_string())
                .await
                .ok(),
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
