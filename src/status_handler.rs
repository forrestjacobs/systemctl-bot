use std::{collections::HashMap, pin::Pin};

use futures::future::{join_all, ready};
use futures::{Stream, StreamExt};
use serenity::all::{ActivityData, Context, EventHandler, Ready};
use serenity::async_trait;
use tokio_stream::StreamMap;

use crate::systemd::Systemd;

pub struct StatusHandler<'a, S: Systemd> {
    pub systemd: &'a S,
    pub units: Vec<String>,
}

impl<S: Systemd> StatusHandler<'_, S> {
    async fn activity_stream<'a>(
        &'a self,
    ) -> Pin<Box<impl Stream<Item = Option<ActivityData>> + use<'a, S>>> {
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
impl<S: Systemd + Sync> EventHandler for StatusHandler<'_, S> {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let mut stream = self.activity_stream().await;
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
                let unit = unit.to_string();
                let mut rx = active_rx.resubscribe();
                Box::pin(async move {
                    let stream: Pin<Box<StatusStream>> = Box::pin(stream! {
                        while let Ok((target, status)) = rx.recv().await {
                            if target == unit {
                                yield Ok(status.to_string())
                            }
                        }
                    });
                    stream
                })
            });
        let handler = StatusHandler {
            systemd: &provider,
            units: vec!["a".to_string(), "b".to_string()],
        };

        let mut stream = handler.activity_stream().await;
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
