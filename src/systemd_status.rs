use async_trait::async_trait;
use async_stream::stream;
use futures::{Stream, StreamExt};
use mockall::automock;
use std::{any::Any, pin::Pin, sync::Arc};
use zbus::{Connection, Result};
use zbus_systemd::systemd1::{ManagerProxy, UnitProxy};

pub type StatusStream = dyn Stream<Item = Result<String>> + Send;

#[automock]
#[async_trait]
pub trait SystemdStatusManager: Any + Sync + Send {
    async fn status_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>>;
}

pub struct SystemdStatusManagerImpl {
    conn: Arc<Connection>,
    client: ManagerProxy<'static>,
}

impl SystemdStatusManagerImpl {
    pub async fn build() -> Result<Self> {
        let conn = Arc::from(Connection::system().await?);
        Ok(SystemdStatusManagerImpl {
            conn: conn.clone(),
            client: ManagerProxy::new(conn.as_ref()).await?,
        })
    }
}

#[async_trait]
impl SystemdStatusManager for SystemdStatusManagerImpl {
    async fn status_stream(&self, unit: &str) -> Result<Pin<Box<StatusStream>>> {
        let unit_name = unit.to_string();
        let path = self.client.load_unit(unit_name.clone()).await?;
        let unit: UnitProxy<'static> = UnitProxy::builder(self.conn.as_ref())
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
