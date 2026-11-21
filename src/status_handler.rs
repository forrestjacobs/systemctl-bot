use core::pin::Pin;
use std::collections::HashMap;

use futures::future::{join_all, ready};
use futures::{Stream, StreamExt as _};
use serenity::all::{ActivityData, Context, EventHandler, Ready};
use serenity::async_trait;
use tokio_stream::StreamMap;
use zbus_systemd::zbus;

use crate::systemd::{StatusStream, Systemd};

pub struct StatusHandler<'handler, S: Systemd> {
    pub systemd: &'handler S,
    pub units: Vec<&'handler str>,
}

impl<S: Systemd> StatusHandler<'_, S> {
    async fn activity_stream(
        &self,
    ) -> zbus::Result<Pin<Box<impl Stream<Item = Option<ActivityData>> + use<'_, S>>>> {
        let streams = self
            .units
            .iter()
            .map(|unit| self.systemd.active_state_stream(unit));
        let streams = join_all(streams)
            .await
            .into_iter()
            .collect::<zbus::Result<Vec<Pin<Box<StatusStream>>>>>()?;
        let streams = StreamMap::from_iter(self.units.iter().zip(streams));

        let mut is_active_by_unit = HashMap::new();
        Ok(Box::pin(streams.filter_map(move |(event_unit, status)| {
            let is_active = status.is_ok_and(|value| value == "active");
            if is_active_by_unit
                .insert(event_unit, is_active)
                .unwrap_or(false)
                == is_active
            {
                return ready(None);
            }

            let active_units: Vec<&str> = self
                .units
                .iter()
                .filter(|unit| is_active_by_unit.get(unit).copied().unwrap_or(false))
                .copied()
                .collect();

            ready(Some(if active_units.is_empty() {
                None
            } else {
                Some(ActivityData::playing(active_units.join(", ")))
            }))
        })))
    }
}

#[async_trait]
impl<S: Systemd + Sync> EventHandler for StatusHandler<'_, S> {
    #[expect(clippy::expect_used, reason = "irrecoverable errors")]
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let mut stream = self
            .activity_stream()
            .await
            .expect("Unable to build activity stream");
        while let Some(activity) = stream.next().await {
            ctx.set_activity(activity);
        }
    }
}

#[cfg(test)]
mod tests {
    use async_stream::stream;
    use tokio::sync::broadcast;

    use crate::systemd::{MockSystemd, StatusStream};

    use super::*;

    #[tokio::test]
    async fn handle_activity_stream() {
        let (active_tx, active_rx) = broadcast::channel::<(&str, &str)>(16);

        let mut provider = MockSystemd::new();
        provider
            .expect_active_state_stream()
            .returning(move |unit| {
                let unit = unit.to_owned();
                let mut rx = active_rx.resubscribe();
                Box::pin(async move {
                    let stream: Pin<Box<StatusStream>> = Box::pin(stream! {
                        while let Ok((target, status)) = rx.recv().await {
                            if target == unit {
                                yield Ok(status.to_owned())
                            }
                        }
                    });
                    Ok(stream)
                })
            });
        let handler = StatusHandler {
            systemd: &provider,
            units: vec!["a", "b"],
        };

        let mut stream = handler.activity_stream().await.unwrap();
        active_tx.send(("a", "active")).unwrap();
        assert_eq!(stream.next().await, Some(Some(ActivityData::playing("a"))));
        active_tx.send(("b", "active")).unwrap();
        assert_eq!(
            stream.next().await,
            Some(Some(ActivityData::playing("a, b")))
        );
        active_tx.send(("b", "active")).unwrap();
        // Nothing happens
        active_tx.send(("b", "inactive")).unwrap();
        assert_eq!(stream.next().await, Some(Some(ActivityData::playing("a"))));
        active_tx.send(("a", "inactive")).unwrap();
        assert_eq!(stream.next().await, Some(None));
    }
}
