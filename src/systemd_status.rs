use async_trait::async_trait;
use futures::{future::join_all, Stream, StreamExt};
use mockall::automock;
use std::{any::Any, pin::Pin};
use zbus::{dbus_proxy, Connection, Result};

pub type StatusStream = dyn Stream<Item = String> + Send;

#[automock]
#[async_trait]
pub trait SystemdStatusManager: Any + Sync + Send {
    async fn status(&self, unit: &str) -> Result<String>;
    async fn status_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>>;
}

impl dyn SystemdStatusManager {
    pub async fn statuses<'a>(
        &self,
        units: &'a Vec<String>,
    ) -> impl Iterator<Item = (&'a str, Result<String>)> {
        let statuses = units.iter().map(|unit| self.status(unit));
        let statuses = join_all(statuses).await;
        units.into_iter().map(|unit| unit.as_str()).zip(statuses)
    }
}

#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Manager {
    #[dbus_proxy(object = "Unit")]
    fn load_unit(&self, name: &str);
}

#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait Unit {
    #[dbus_proxy(property)]
    fn active_state(&self) -> Result<String>;
}

pub struct SystemdStatusManagerImpl {
    client: ManagerProxy<'static>,
}

impl SystemdStatusManagerImpl {
    pub async fn build() -> Result<Self> {
        let conn = Connection::system().await?;
        Ok(SystemdStatusManagerImpl {
            client: ManagerProxy::new(&conn).await?,
        })
    }
}

#[async_trait]
impl SystemdStatusManager for SystemdStatusManagerImpl {
    async fn status(&self, unit: &str) -> Result<String> {
        let unit = self.client.load_unit(unit).await?;
        unit.active_state().await
    }

    async fn status_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>> {
        let unit_name = unit.to_string();
        let unit = self.client.load_unit(unit).await?;
        let stream = unit.receive_active_state_changed().await;
        Ok(Box::pin(stream.map(move |_| unit_name.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::Error;

    #[tokio::test]
    async fn get_statuses() {
        let mut manager = MockSystemdStatusManager::new();
        manager.expect_status().returning(|unit| {
            if unit == "invalid.service" {
                Err(Error::InvalidReply)
            } else {
                Ok(unit.strip_suffix(".service").unwrap_or(unit).into())
            }
        });

        let mock: Box<dyn SystemdStatusManager> = Box::from(manager);
        let units = vec![
            String::from("active.service"),
            String::from("inactive.service"),
            String::from("invalid.service"),
        ];
        let statuses: Vec<(&str, Result<String>)> = mock.statuses(&units).await.collect();
        assert_eq!(
            statuses,
            vec![
                ("active.service", Ok(String::from("active"))),
                ("inactive.service", Ok(String::from("inactive"))),
                ("invalid.service", Err(Error::InvalidReply)),
            ]
        );
    }
}
