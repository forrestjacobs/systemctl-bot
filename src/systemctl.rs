use futures::future::join_all;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::io;
use std::process::{ExitStatus, Output};
use tokio::process::Command;
use tokio_stream::StreamMap;
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
    fn get_unit(&self, name: String);
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

pub async fn get_manager<'a>() -> zbus::Result<ManagerProxy<'a>> {
    let conn = Connection::system().await?;
    let client = ManagerProxy::new(&conn).await?;
    Ok(client)
}

pub async fn start<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("start", &unit).await
}

pub async fn stop<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("stop", &unit).await
}

pub async fn restart<S: AsRef<OsStr>>(unit: S) -> Result<(), SystemctlError> {
    systemctl_do("restart", &unit).await
}

pub async fn status<S: AsRef<OsStr>>(unit: S) -> Result<String, SystemctlError> {
    let output = Command::new("systemctl")
        .arg("is-active")
        .arg(&unit)
        .output()
        .await?;
    let output = String::from(to_str(output.stdout).trim_end_matches('\n'));
    Ok(output)
}

async fn status_with_name(unit: &str) -> (&str, Result<String, SystemctlError>) {
    let output = status(unit).await;
    (unit, output)
}

pub async fn statuses<'a, I: Iterator<Item = &'a str>>(
    units: I,
) -> Vec<(&'a str, Result<String, SystemctlError>)> {
    join_all(units.map(|unit| status_with_name(unit))).await
}

async fn get_active_state_stream<'a, 'b>(
    manager: &ManagerProxy<'a>,
    unit_name: &'b str,
) -> zbus::Result<(&'b str, PropertyStream<'a, String>)> {
    let unit = manager.get_unit(String::from(unit_name)).await?;
    let stream = unit.receive_active_state_changed().await;
    Ok((unit_name, stream))
}

pub async fn get_active_state_by_unit_stream<'a>(
    unit_names: Vec<&str>,
) -> zbus::Result<StreamMap<&str, PropertyStream<'a, String>>> {
    let manager = get_manager().await?;
    Ok(join_all(
        unit_names
            .into_iter()
            .map(|unit_name| get_active_state_stream(&manager, unit_name)),
    )
    .await
    .into_iter()
    .collect::<zbus::Result<StreamMap<&str, PropertyStream<String>>>>()?)
}
