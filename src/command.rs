use crate::systemctl::{restart, start, stop};
use crate::systemd_status::SystemdStatusManager;
use crate::units::{UnitPermissions, Units, UnitsTrait};
use anyhow::bail;
use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum UserCommand {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
    SingleStatus { unit: String },
    MultiStatus,
}

fn ensure_allowed(units: &Units, unit: &str, permissions: UnitPermissions) -> anyhow::Result<()> {
    if !units
        .get(unit)
        .map_or(false, |unit| unit.contains(permissions))
    {
        bail!("Command is not allowed");
    }
    Ok(())
}

impl UserCommand {
    pub async fn run(
        &self,
        units: &Units,
        systemd_status_manager: &SystemdStatusManager,
    ) -> anyhow::Result<String> {
        match self {
            UserCommand::Start { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Start)?;
                start(unit).await?;
                Ok(format!("Started {}", unit))
            }
            UserCommand::Stop { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Stop)?;
                stop(unit).await?;
                Ok(format!("Stopped {}", unit))
            }
            UserCommand::Restart { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Stop | UnitPermissions::Start)?;
                restart(unit).await?;
                Ok(format!("Restarted {}", unit))
            }
            UserCommand::SingleStatus { unit } => {
                ensure_allowed(units, unit, UnitPermissions::Status)?;
                Ok(systemd_status_manager.status(unit).await?)
            }
            UserCommand::MultiStatus => {
                let mut status_lines = systemd_status_manager
                    .statuses(units.with_permissions(UnitPermissions::Status))
                    .await
                    .map(|(unit, status)| (unit, status.unwrap_or_else(|err| format!("{}", err))))
                    .filter(|(_, status)| status != "inactive")
                    .map(|(unit, status)| format!("{}: {}", unit, status))
                    .peekable();
                let response = if status_lines.peek().is_none() {
                    String::from("Nothing is active")
                } else {
                    status_lines.join("\n")
                };
                Ok(response)
            }
        }
    }
}
