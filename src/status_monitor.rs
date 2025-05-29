use super::systemd_status::SystemdStatusManager;
use crate::systemd_status::StatusStream;
use async_stream::stream;
use async_trait::async_trait;
use futures::future::join_all;
use futures::{Stream, StreamExt};
use poise::serenity_prelude::all::{ActivityData, Context};
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::StreamMap;
use zbus::Result;

#[async_trait]
pub trait StatusMonitor: Any + Send + Sync {
    async fn monitor(&self, ctx: &Context);
}

pub struct StatusMonitorImpl<M: SystemdStatusManager> {
    pub units: Vec<String>,
    pub systemd_status_manager: M,
}

impl<M: SystemdStatusManager> StatusMonitorImpl<M> {
    async fn get_stream(
        &self,
    ) -> Result<Pin<Box<impl Stream<Item = Option<String>> + use<'_, M>>>> {
        let status_streams = self
            .units
            .iter()
            .map(|u| self.systemd_status_manager.status_stream(u));
        let status_streams = join_all(status_streams)
            .await
            .into_iter()
            .collect::<Result<Vec<Pin<Box<StatusStream>>>>>()?;
        let mut status_streams =
            StreamMap::from_iter(self.units.iter().map(String::as_str).zip(status_streams));

        let mut is_active_by_unit = HashMap::new();
        Ok(Box::pin(stream! {
            while let Some((unit, status)) = status_streams.next().await {
                let is_active = status.map_or(false, |v| v == "active");
                if is_active_by_unit.insert(unit, is_active) == Some(is_active) {
                    continue;
                }

                let active_units: Vec<&str> = self.units
                    .iter()
                    .filter(|unit| *is_active_by_unit.get(unit.as_str()).unwrap_or(&false))
                    .map(String::as_str)
                    .collect();

                yield if active_units.is_empty() {
                    None
                } else {
                    Some(active_units.join(", "))
                };
            }
        }))
    }
}

#[async_trait]
impl<M: SystemdStatusManager> StatusMonitor for StatusMonitorImpl<M> {
    async fn monitor(&self, ctx: &Context) {
        let mut stream = self.get_stream().await.unwrap();
        while let Some(status) = stream.next().await {
            ctx.set_activity(status.map(ActivityData::playing));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systemd_status::MockSystemdStatusManager;
    use async_stream::stream;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_update_activity_stream() {
        let units = vec![
            "a.service".to_string(),
            "b.service".to_string(),
            "c.service".to_string(),
        ];

        let (tx, rx) = broadcast::channel::<(String, String)>(4);
        let mut manager = MockSystemdStatusManager::new();
        manager
            .expect_status_stream()
            .times(3)
            .returning(move |unit| {
                let unit = unit.to_string();
                let mut rx = rx.resubscribe();
                Ok(Box::pin(stream! {
                    loop {
                        let (target, status) = rx.recv().await.unwrap();
                        if target == unit {
                            yield Ok(status)
                        }
                    }
                }))
            });
        let monitor = StatusMonitorImpl {
            units,
            systemd_status_manager: manager,
        };
        let mut stream = monitor.get_stream().await.unwrap();
        tx.send(("a.service".to_string(), "active".to_string()))
            .unwrap();
        assert_eq!(stream.next().await.unwrap(), Some("a.service".to_string()));
        tx.send(("c.service".to_string(), "active".to_string())).unwrap();
        assert_eq!(stream.next().await.unwrap(), Some("a.service, c.service".to_string()));
        tx.send(("a.service".to_string(), "deactivating".to_string())).unwrap();
        assert_eq!(stream.next().await.unwrap(), Some("c.service".to_string()));
    }
}
