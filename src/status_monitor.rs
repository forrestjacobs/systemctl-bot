use super::systemd_status::SystemdStatusManager;
use crate::config::{Command, Config};
use async_trait::async_trait;
use futures::future::join_all;
use futures::StreamExt;
use poise::serenity_prelude::all::{ActivityData, Context};
use shaku::{Component, Interface};
use std::sync::Arc;
use tokio_stream::StreamMap;
use zbus::PropertyStream;

#[async_trait]
pub trait StatusMonitor: Interface {
    async fn monitor(&self, ctx: &Context);
}

#[derive(Component)]
#[shaku(interface = StatusMonitor)]
pub struct StatusMonitorImpl {
    #[shaku(inject)]
    config: Arc<dyn Config>,
    #[shaku(inject)]
    systemd_status_manager: Arc<dyn SystemdStatusManager>,
}

impl StatusMonitorImpl {
    async fn update_activity_stream(
        &self,
    ) -> Result<StreamMap<&str, PropertyStream<'_, String>>, zbus::Error> {
        let streams = self.config.units[&Command::Status]
            .iter()
            .map(|u| self.systemd_status_manager.status_stream(u));
        let streams = join_all(streams)
            .await
            .into_iter()
            .collect::<Result<Vec<PropertyStream<String>>, zbus::Error>>()?;
        Ok(self.config.units[&Command::Status]
            .iter()
            .map(|u| u.as_str())
            .zip(streams)
            .collect::<StreamMap<&str, PropertyStream<String>>>())
    }

    async fn update_activity(&self, ctx: &Context) {
        let units = self.config.units[&Command::Status]
            .iter()
            .map(|u| u.as_str());
        let statuses = self.systemd_status_manager.statuses(units).await;
        let active_units = statuses
            .into_iter()
            .filter(|(_, status)| status.as_ref().map_or(false, |status| status == "active"))
            .map(|(unit, _)| unit)
            .collect::<Vec<&str>>();

        if active_units.is_empty() {
            ctx.reset_presence();
        } else {
            let activity = ActivityData::playing(active_units.join(", "));
            ctx.set_activity(Some(activity));
        }
    }
}

#[async_trait]
impl StatusMonitor for StatusMonitorImpl {
    async fn monitor(&self, ctx: &Context) {
        let mut stream = self.update_activity_stream().await.unwrap();
        self.update_activity(ctx).await;
        while stream.next().await.is_some() {
            self.update_activity(ctx).await;
        }
    }
}
