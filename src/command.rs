use crate::systemctl::{restart, start, stop};
use crate::systemd_status::SystemdStatusManager;
use crate::units::{UnitPermissions, Units, UnitsTrait};
use anyhow::bail;
use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum UserCommand<'a> {
    Start(&'a str),
    Stop(&'a str),
    Restart(&'a str),
    SingleStatus(&'a str),
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

impl<'a> UserCommand<'a> {
    fn check_permissions(&self, units: &Units) -> anyhow::Result<()> {
        match self {
            UserCommand::Start(unit) => ensure_allowed(units, unit, UnitPermissions::Start),
            UserCommand::Stop(unit) => ensure_allowed(units, unit, UnitPermissions::Stop),
            UserCommand::Restart(unit) => {
                ensure_allowed(units, unit, UnitPermissions::Stop | UnitPermissions::Start)
            }
            UserCommand::SingleStatus(unit) => ensure_allowed(units, unit, UnitPermissions::Status),
            _ => Ok(()),
        }
    }

    pub async fn run(
        &self,
        units: &Units,
        systemd_status_manager: &SystemdStatusManager,
    ) -> anyhow::Result<String> {
        self.check_permissions(units)?;
        match self {
            UserCommand::Start(unit) => {
                start(unit).await?;
                Ok(format!("Started {}", unit))
            }
            UserCommand::Stop(unit) => {
                stop(unit).await?;
                Ok(format!("Stopped {}", unit))
            }
            UserCommand::Restart(unit) => {
                restart(unit).await?;
                Ok(format!("Restarted {}", unit))
            }
            UserCommand::SingleStatus(unit) => Ok(systemd_status_manager.status(unit).await?),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_unit_fixture() -> Units {
        Units::from([
            ("000.service".to_string(), UnitPermissions::empty()),
            ("001.service".to_string(), UnitPermissions::Status),
            ("010.service".to_string(), UnitPermissions::Stop),
            (
                "011.service".to_string(),
                UnitPermissions::Stop | UnitPermissions::Status,
            ),
            ("100.service".to_string(), UnitPermissions::Start),
            (
                "101.service".to_string(),
                UnitPermissions::Start | UnitPermissions::Status,
            ),
            (
                "110.service".to_string(),
                UnitPermissions::Start | UnitPermissions::Stop,
            ),
            ("111.service".to_string(), UnitPermissions::all()),
        ])
    }

    #[test]
    fn check_permissions() {
        let units = get_unit_fixture();

        assert!(UserCommand::Start("110.service")
            .check_permissions(&units)
            .is_ok());
        assert!(UserCommand::Start("011.service")
            .check_permissions(&units)
            .is_err());

        assert!(UserCommand::Stop("011.service")
            .check_permissions(&units)
            .is_ok());
        assert!(UserCommand::Stop("101.service")
            .check_permissions(&units)
            .is_err());

        assert!(UserCommand::Restart("111.service")
            .check_permissions(&units)
            .is_ok());
        assert!(UserCommand::Restart("101.service")
            .check_permissions(&units)
            .is_err());
        assert!(UserCommand::Restart("011.service")
            .check_permissions(&units)
            .is_err());

        assert!(UserCommand::SingleStatus("101.service")
            .check_permissions(&units)
            .is_ok());
        assert!(UserCommand::SingleStatus("110.service")
            .check_permissions(&units)
            .is_err());

        assert!(UserCommand::MultiStatus.check_permissions(&units).is_ok());
    }
}
