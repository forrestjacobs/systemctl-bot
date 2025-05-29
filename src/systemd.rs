use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use mockall::automock;
use std::{any::Any, pin::Pin, sync::Arc};
use zbus::{Connection};
use anyhow::{bail, Result};
use zbus_systemd::systemd1::{ManagerProxy, UnitProxy};

pub type StatusStream = dyn Stream<Item = zbus::Result<String>> + Send;

#[derive(Debug, PartialEq)]
pub enum UnitVerb {
    Start,
    Stop,
    Restart,
}

#[automock]
#[async_trait]
pub trait SystemdManager: Any + Send + Sync {
    async fn run(&self, verb: UnitVerb, unit: &str) -> Result<()>;
    async fn active_state_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>>;
}

pub struct SystemdManagerImpl {
    conn: Arc<Connection>,
    client: ManagerProxy<'static>,
}

impl SystemdManagerImpl {
    pub async fn build() -> Result<Self> {
        let conn = Arc::from(Connection::system().await?);
        Ok(SystemdManagerImpl {
            conn: conn.clone(),
            client: ManagerProxy::new(conn.as_ref()).await?,
        })
    }
}

#[async_trait]
impl SystemdManager for SystemdManagerImpl {
    async fn run(&self, verb: UnitVerb, unit: &str) -> Result<()> {
        let mut job_stream = self.client.receive_job_removed().await?;
        let job_path = match verb {
            UnitVerb::Start => {
                self.client
                    .start_unit(unit.to_string(), "replace".to_string())
                    .await
            }
            UnitVerb::Stop => {
                self.client
                    .stop_unit(unit.to_string(), "replace".to_string())
                    .await
            }
            UnitVerb::Restart => {
                self.client
                    .restart_unit(unit.to_string(), "replace".to_string())
                    .await
            }
        }?;
        while let Some(event) = job_stream.next().await {
            let args = event.args()?;
            if args.job == job_path {
                return match args.result.as_str() {
                    "done" => Ok(()),
                    _ => bail!(args.result),
                };
            }
        }
        bail!("Job stream closed")
    }

    async fn active_state_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>> {
        let path = self.client.load_unit(unit.to_string()).await?;
        let unit = UnitProxy::builder(self.conn.as_ref())
            .path(path)?
            .build()
            .await?;
        let mut prop_stream = unit.receive_active_state_changed().await;
        Ok(Box::pin(stream! {
            while let Some(event) = prop_stream.next().await {
                yield event.get().await
            }
        }))
    }
}
