use futures::future::join_all;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{ExitStatus, Output};
use tokio::process::Command;
use zbus::{dbus_proxy, Connection, PropertyStream};

#[derive(Debug)]
pub enum SystemctlError {
    IoError(io::Error),
    NonZeroExit { status: ExitStatus, stderr: String },
}

impl Display for SystemctlError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SystemctlError::IoError(e) => write!(f, "{}", e),
            SystemctlError::NonZeroExit { status, stderr } => {
                write!(f, "systemctl failed with {}\n\n{}", status, stderr)
            }
        }
    }
}

impl Error for SystemctlError {}

impl From<io::Error> for SystemctlError {
    fn from(error: io::Error) -> Self {
        SystemctlError::IoError(error)
    }
}

fn to_str(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

impl From<Output> for SystemctlError {
    fn from(output: Output) -> Self {
        SystemctlError::NonZeroExit {
            status: output.status,
            stderr: to_str(output.stderr),
        }
    }
}

#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Manager {
    #[dbus_proxy(object = "Unit")]
    fn get_unit(&self, name: &str);
}

#[dbus_proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait Unit {
    #[dbus_proxy(property)]
    fn active_state(&self) -> zbus::Result<String>;
}

async fn systemctl_do<S: AsRef<OsStr>, T: AsRef<OsStr>>(
    verb: S,
    unit: T,
) -> Result<(), SystemctlError> {
    let output = Command::new("systemctl")
        .arg(verb)
        .arg(unit)
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SystemctlError::from(output))
    }
}

pub struct SystemctlManager<'a> {
    client: ManagerProxy<'a>,
}

impl SystemctlManager<'_> {
    pub async fn new<'a>() -> zbus::Result<SystemctlManager<'a>> {
        let conn = Connection::system().await?;
        let client = ManagerProxy::new(&conn).await?;
        Ok(SystemctlManager { client })
    }

    pub async fn start<S: AsRef<OsStr>>(&self, unit: S) -> Result<(), SystemctlError> {
        systemctl_do("start", &unit).await
    }

    pub async fn stop<S: AsRef<OsStr>>(&self, unit: S) -> Result<(), SystemctlError> {
        systemctl_do("stop", &unit).await
    }

    pub async fn restart<S: AsRef<OsStr>>(&self, unit: S) -> Result<(), SystemctlError> {
        systemctl_do("restart", &unit).await
    }

    pub async fn status(&self, unit: &str) -> zbus::Result<String> {
        let unit = self.client.get_unit(unit).await?;
        unit.active_state().await
    }

    pub async fn status_stream(&self, unit: &str) -> zbus::Result<PropertyStream<'_, String>> {
        Ok(self
            .client
            .get_unit(unit)
            .await?
            .receive_active_state_changed()
            .await)
    }
}

async fn status_with_name<'a, 'b>(
    systemctl: &SystemctlManager<'a>,
    unit: &'b str,
) -> (&'b str, zbus::Result<String>) {
    (unit, systemctl.status(unit).await)
}

pub async fn statuses<'a, 'b, I: Iterator<Item = &'b str>>(
    systemctl: &SystemctlManager<'a>,
    units: I,
) -> Vec<(&'b str, zbus::Result<String>)> {
    let statuses = units.map(|unit| status_with_name(systemctl, unit));
    join_all(statuses).await
}
