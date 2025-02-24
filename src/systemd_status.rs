use async_trait::async_trait;
use futures::future::join_all;
use shaku::{Component, Interface};
use zbus::{dbus_proxy, Connection, Error, PropertyStream};

#[async_trait]
pub trait SystemdStatusManager: Interface {
    async fn status(&self, unit: &str) -> Result<String, Error>;
    async fn status_stream(&self, unit: &str) -> Result<PropertyStream<'_, String>, Error>;
}

impl dyn SystemdStatusManager {
    pub async fn statuses<'a, I: Iterator<Item = &'a str>>(
        &self,
        units: I,
    ) -> Vec<(&'a str, Result<String, Error>)> {
        let units: Vec<&str> = units.collect();
        let statuses = units.iter().map(|unit| self.status(unit));
        let statuses = join_all(statuses).await;
        units.into_iter().zip(statuses).collect()
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

#[derive(Component)]
#[shaku(interface = SystemdStatusManager)]
pub struct SystemdStatusManagerImpl {
    client: ManagerProxy<'static>,
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

pub async fn make_params() -> Result<SystemdStatusManagerImplParameters, Error> {
    let conn = Connection::system().await?;
    Ok(SystemdStatusManagerImplParameters {
        client: ManagerProxy::new(&conn).await?,
    })
}
