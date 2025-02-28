use async_trait::async_trait;
use futures::future::join_all;
use std::any::Any;
use zbus::{dbus_proxy, Connection, Error, PropertyStream};

#[async_trait]
pub trait SystemdStatusManager: Any + Sync + Send {
    async fn status(&self, unit: &str) -> Result<String, Error>;
    async fn status_stream(&self, unit: &str) -> Result<PropertyStream<'_, String>, Error>;
}

impl dyn SystemdStatusManager {
    pub async fn statuses<'a>(
        &self,
        units: &'a Vec<String>,
    ) -> impl Iterator<Item = (&'a str, Result<String, Error>)> {
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
    fn active_state(&self) -> zbus::Result<String>;
}

pub struct SystemdStatusManagerImpl {
    client: ManagerProxy<'static>,
}

impl SystemdStatusManagerImpl {
    pub async fn build() -> Result<Self, Error> {
        let conn = Connection::system().await?;
        Ok(SystemdStatusManagerImpl {
            client: ManagerProxy::new(&conn).await?,
        })
    }
}

#[async_trait]
impl SystemdStatusManager for SystemdStatusManagerImpl {
    async fn status(&self, unit: &str) -> Result<String, Error> {
        let unit = self.client.load_unit(unit).await?;
        unit.active_state().await
    }

    async fn status_stream(&self, unit: &str) -> Result<PropertyStream<'_, String>, Error> {
        let unit = self.client.load_unit(unit).await?;
        Ok(unit.receive_active_state_changed().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSystemdStatusManager {}

    #[async_trait]
    impl SystemdStatusManager for MockSystemdStatusManager {
        async fn status(&self, unit: &str) -> Result<String, Error> {
            if unit == "invalid.service" {
                Err(Error::InvalidReply)
            } else {
                Ok(unit.strip_suffix(".service").unwrap_or(unit).into())
            }
        }
        async fn status_stream(&self, _unit: &str) -> Result<PropertyStream<'_, String>, Error> {
            todo!()
        }
    }

    #[tokio::test]
    async fn get_statuses() {
        let mock: Box<dyn SystemdStatusManager> = Box::from(MockSystemdStatusManager {});
        let units = vec![
            String::from("active.service"),
            String::from("inactive.service"),
            String::from("invalid.service"),
        ];
        let statuses: Vec<(&str, Result<String, Error>)> = mock.statuses(&units).await.collect();
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
