use core::error::Error;
use core::fmt::{self, Display, Formatter};
use core::pin::Pin;
use futures::{Stream, StreamExt as _};
use mockall::automock;
use serenity::async_trait;
use zbus_systemd::systemd1::{JobRemoved, UnitProxy};
use zbus_systemd::zbus::OwnedObjectPath;
use zbus_systemd::{
    systemd1::ManagerProxy,
    zbus::{self, Connection},
};

use crate::command::Command;

#[derive(Debug)]
pub enum SystemdRunError {
    JobStreamClosed,
    UnexpectedJobResult(String),
    ZbusError(zbus::Error),
}

impl Display for SystemdRunError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            SystemdRunError::JobStreamClosed => write!(f, "Job stream closed"),
            SystemdRunError::UnexpectedJobResult(ref result) => {
                write!(f, "Unexpected job result {result}")
            }
            SystemdRunError::ZbusError(ref err) => write!(f, "{err}"),
        }
    }
}

impl From<zbus::Error> for SystemdRunError {
    fn from(error: zbus::Error) -> Self {
        SystemdRunError::ZbusError(error)
    }
}

impl Error for SystemdRunError {}

pub struct SystemdImpl {
    client: ManagerProxy<'static>,
    conn: Connection,
}

impl SystemdImpl {
    pub async fn new() -> zbus::Result<Self> {
        let conn = Connection::system().await?;
        Ok(Self {
            client: ManagerProxy::new(&conn).await?,
            conn,
        })
    }
}

pub type StatusStream = dyn Stream<Item = zbus::Result<String>> + Send;

#[async_trait]
#[automock]
pub trait Systemd {
    async fn active_state_stream(&self, unit: &str) -> zbus::Result<Pin<Box<StatusStream>>>;
    async fn run(&self, command: &Command, unit: &str) -> Result<(), SystemdRunError>;
}

#[async_trait]
impl Systemd for SystemdImpl {
    async fn active_state_stream(&self, unit: &str) -> zbus::Result<Pin<Box<StatusStream>>> {
        active_state_stream_internal(&self.conn, &self.client, unit.to_owned()).await
    }

    async fn run(&self, command: &Command, unit: &str) -> Result<(), SystemdRunError> {
        let job_stream = self.client.receive_job_removed().await?;
        let path = run_internal(&self.client, command, unit).await?;
        wait_for_job(Box::pin(job_stream), path).await
    }
}

async fn run_internal(
    client: &ManagerProxy<'static>,
    command: &Command,
    unit: &str,
) -> zbus::Result<OwnedObjectPath> {
    let unit = unit.to_owned();
    let mode = "replace".to_owned();
    match *command {
        Command::Start => client.start_unit(unit, mode).await,
        Command::Stop => client.stop_unit(unit, mode).await,
        Command::Restart => client.restart_unit(unit, mode).await,
    }
}

async fn wait_for_job<S>(
    mut job_stream: Pin<Box<S>>,
    path: OwnedObjectPath,
) -> Result<(), SystemdRunError>
where
    S: Stream<Item = JobRemoved>,
{
    while let Some(event) = job_stream.next().await {
        let args = event.args()?;
        if args.job == path {
            return match args.result.as_str() {
                "done" => Ok(()),
                result => Err(SystemdRunError::UnexpectedJobResult(result.to_owned())),
            };
        }
    }
    Err(SystemdRunError::JobStreamClosed)
}

async fn active_state_stream_internal(
    conn: &Connection,
    client: &ManagerProxy<'static>,
    unit: String,
) -> zbus::Result<Pin<Box<StatusStream>>> {
    let path = client.load_unit(unit).await?;
    let builder = UnitProxy::builder(conn).path(path);
    let builder = builder.build().await?;
    let stream = builder.receive_active_state_changed().await;
    Ok(Box::pin(stream.then(async |event| event.get().await)))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use futures::stream;
    use zbus_systemd::zbus::Message;

    use super::*;

    fn make_job_removed_message(path: &OwnedObjectPath, result: &str) -> zbus::Result<Message> {
        zbus::Message::signal(
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
            "JobRemoved",
        )
        .build(&(
            0_u32, // This should match the number in the path, but in practice we don't use this arg
            path.clone(),
            "demo.service".to_owned(),
            result.to_owned(),
        ))
    }

    #[test]
    fn display_systemd_run_errors() {
        assert_eq!(
            SystemdRunError::UnexpectedJobResult("weird".to_owned()).to_string(),
            "Unexpected job result weird"
        );
        assert_eq!(
            SystemdRunError::JobStreamClosed.to_string(),
            "Job stream closed"
        );
        assert_eq!(
            SystemdRunError::from(zbus::Error::IncorrectType).to_string(),
            "incorrect type"
        );
    }

    #[tokio::test]
    async fn wait_for_successful_job() {
        let path = OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/123").unwrap();
        let message = make_job_removed_message(&path, "done").unwrap();
        let event = JobRemoved::from_message(message).unwrap();
        assert_matches!(
            wait_for_job(Box::pin(stream::iter([event])), path).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn wait_for_failed_job() {
        let path = OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/123").unwrap();
        let message = make_job_removed_message(&path, "failed").unwrap();
        let event = JobRemoved::from_message(message).unwrap();
        assert_matches!(
            wait_for_job(Box::pin(stream::iter([event])), path).await,
            Err(SystemdRunError::UnexpectedJobResult(result)) if result == "failed"
        );
    }

    #[tokio::test]
    async fn wait_for_closed_job_stream() {
        let path = OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/123").unwrap();
        assert_matches!(
            wait_for_job(Box::pin(stream::iter([])), path).await,
            Err(SystemdRunError::JobStreamClosed)
        );
    }
}
