use std::sync::Arc;
use std::{collections::HashMap, pin::Pin};

use futures::future::{join_all, ready};
use futures::{Stream, StreamExt};
use serenity::all::{ActivityData, Context, EventHandler, Ready};
use serenity::async_trait;
use tokio_stream::StreamMap;

use crate::systemd::Systemd;

pub struct StatusHandler {
    pub systemd: Arc<Systemd>,
    pub units: Vec<String>,
}

impl StatusHandler {
    async fn activity_stream<'a>(
        &'a self,
    ) -> Pin<Box<impl Stream<Item = Option<ActivityData>> + use<'a>>> {
        let streams = StreamMap::from_iter(
            self.units.iter().zip(
                join_all(
                    self.units
                        .iter()
                        .map(|u| self.systemd.active_state_stream(u)),
                )
                .await,
            ),
        );

        let mut is_active_by_unit = HashMap::new();
        Box::pin(streams.filter_map(move |(unit, status)| {
            let is_active = status.is_ok_and(|v| v == "active");
            if is_active_by_unit.insert(unit, is_active).unwrap_or(false) == is_active {
                return ready(None);
            }

            let active_units: Vec<&str> = self
                .units
                .iter()
                .filter(|unit| is_active_by_unit.get(unit).copied().unwrap_or(false))
                .map(String::as_str)
                .collect();

            ready(Some(if active_units.is_empty() {
                None
            } else {
                Some(ActivityData::playing(active_units.join(", ")))
            }))
        }))
    }
}

#[async_trait]
impl EventHandler for StatusHandler {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let mut stream = self.activity_stream().await;
        while let Some(activity) = stream.next().await {
            ctx.set_activity(activity);
        }
    }
}
