use super::systemd_status::SystemdStatusManager;
use crate::config::{Command, UnitCollection};
use crate::systemd_status::StatusStream;
use async_trait::async_trait;
use futures::future::join_all;
use futures::stream::{select_all, SelectAll};
use futures::StreamExt;
use poise::serenity_prelude::all::{ActivityData, Context};
use std::pin::Pin;
use std::{any::Any, sync::Arc};
use zbus::Result;

#[async_trait]
pub trait StatusMonitor: Any + Send + Sync {
    async fn monitor(&self, ctx: &Context);
}

pub struct StatusMonitorImpl {
    pub units: Arc<UnitCollection>,
    pub systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

impl StatusMonitorImpl {
    async fn update_activity_stream(&self) -> Result<SelectAll<Pin<Box<StatusStream>>>> {
        let streams = self.units[&Command::Status]
            .iter()
            .map(|u| self.systemd_status_manager.status_stream(u));
        let streams = join_all(streams).await;
        let streams = streams
            .into_iter()
            .collect::<Result<Vec<Pin<Box<StatusStream>>>>>()?;
        Ok(select_all(streams))
    }

    async fn get_activity(&self) -> Option<String> {
        let active_units = self
            .systemd_status_manager
            .statuses(&self.units[&Command::Status])
            .await
            .filter(|(_, status)| status == &Ok(String::from("active")))
            .map(|(unit, _)| unit)
            .collect::<Vec<&str>>();

        if active_units.is_empty() {
            None
        } else {
            Some(active_units.join(", "))
        }
    }

    fn set_activity(ctx: &Context, value: &Option<String>) {
        ctx.set_activity(value.as_ref().map(ActivityData::playing))
    }
}

#[async_trait]
impl StatusMonitor for StatusMonitorImpl {
    async fn monitor(&self, ctx: &Context) {
        let mut stream = self.update_activity_stream().await.unwrap();
        let mut activity = self.get_activity().await;
        Self::set_activity(ctx, &activity);
        while stream.next().await.is_some() {
            let updated_activity = self.get_activity().await;
            if activity == updated_activity {
                continue;
            }
            activity = updated_activity;
            Self::set_activity(ctx, &activity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systemd_status::MockSystemdStatusManager;
    use std::collections::HashMap;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_update_activity_stream() {
        let units = Arc::from(UnitCollection::from(HashMap::from([(
            Command::Status,
            vec![
                "a.service".to_string(),
                "b.service".to_string(),
                "c.service".to_string(),
            ],
        )])));

        let (tx, rx) = broadcast::channel::<String>(4);
        let mut manager = MockSystemdStatusManager::new();
        manager
            .expect_status_stream()
            .times(3)
            .returning(move |unit| {
                let unit = unit.to_string();
                let mut rx = rx.resubscribe();
                Ok(Box::pin(futures::stream::once(async move {
                    loop {
                        if rx.recv().await.map_or(false, |u| &u == &unit) {
                            return unit;
                        }
                    }
                })))
            });
        let monitor = StatusMonitorImpl {
            units,
            systemd_status_manager: Arc::from(manager),
        };
        let mut stream = monitor.update_activity_stream().await.unwrap();
        tx.send("a.service".to_string()).unwrap();
        assert_eq!(stream.next().await, Some("a.service".to_string()));
        tx.send("c.service".to_string()).unwrap();
        assert_eq!(stream.next().await, Some("c.service".to_string()));
        tx.send("b.service".to_string()).unwrap();
        assert_eq!(stream.next().await, Some("b.service".to_string()));
        tx.send("z.service".to_string()).unwrap();
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test]
    async fn test_update_activity() {
        let units = Arc::from(UnitCollection::from(HashMap::from([(
            Command::Status,
            vec![
                "inactive.service".to_string(),
                "active.service".to_string(),
                "invalid.service".to_string(),
            ],
        )])));
        let mut manager = MockSystemdStatusManager::new();
        manager.expect_status().returning(|unit| {
            if unit == "invalid.service" {
                Err(zbus::Error::InvalidReply)
            } else {
                Ok(unit.strip_suffix(".service").unwrap_or(unit).into())
            }
        });
        let monitor = StatusMonitorImpl {
            units,
            systemd_status_manager: Arc::from(manager),
        };
        assert_eq!(
            monitor.get_activity().await,
            Some("active.service".to_string())
        );
    }
}
