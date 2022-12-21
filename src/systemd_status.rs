use futures::future::join_all;
use zbus::{dbus_proxy, Connection, Error, PropertyStream};

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

pub struct SystemdStatusManager<'a> {
    client: ManagerProxy<'a>,
}

impl SystemdStatusManager<'_> {
    pub async fn new<'a>() -> Result<SystemdStatusManager<'a>, Error> {
        let conn = Connection::system().await?;
        let client = ManagerProxy::new(&conn).await?;
        Ok(SystemdStatusManager { client })
    }

    pub async fn status(&self, unit: &str) -> Result<String, Error> {
        let unit = self.client.load_unit(unit).await?;
        unit.active_state().await
    }

    pub async fn status_stream(&self, unit: &str) -> Result<PropertyStream<'_, String>, Error> {
        Ok(self
            .client
            .load_unit(unit)
            .await?
            .receive_active_state_changed()
            .await)
    }
}

async fn status_with_name<'a, 'b>(
    manager: &SystemdStatusManager<'a>,
    unit: &'b str,
) -> (&'b str, Result<String, Error>) {
    (unit, manager.status(unit).await)
}

pub async fn statuses<'a, 'b, I: Iterator<Item = &'b str>>(
    manager: &SystemdStatusManager<'a>,
    units: I,
) -> Vec<(&'b str, Result<String, Error>)> {
    let statuses = units.map(|unit| status_with_name(manager, unit));
    join_all(statuses).await
}
