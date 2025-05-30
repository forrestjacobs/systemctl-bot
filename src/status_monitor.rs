use super::systemd::SystemdManager;
use crate::systemd::StatusStream;
use anyhow::Result;
use async_stream::stream;
use futures::future::join_all;
use futures::{Stream, StreamExt};
use poise::serenity_prelude::all::{ActivityData, Context};
use std::collections::HashMap;
use std::pin::Pin;
use tokio_stream::StreamMap;

async fn get_status_stream<'a, S: SystemdManager + ?Sized>(
    units: &'a [String],
    systemd: &'a S,
) -> Result<Pin<Box<impl Stream<Item = Option<String>> + use<'a, S>>>> {
    let streams = units.iter().map(|u| systemd.active_state_stream(u));
    let streams = join_all(streams)
        .await
        .into_iter()
        .collect::<Result<Vec<Pin<Box<StatusStream>>>>>()?;
    let mut streams = StreamMap::from_iter(units.iter().zip(streams));

    let mut is_active_by_unit = HashMap::new();
    Ok(Box::pin(stream! {
        while let Some((unit, status)) = streams.next().await {
            let is_active = status.is_ok_and(|v| v == "active");
            if is_active_by_unit.insert(unit, is_active) == Some(is_active) {
                continue;
            }

            let active_units: Vec<&str> = units
                .iter()
                .filter(|unit| *is_active_by_unit.get(unit).unwrap_or(&false))
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

pub async fn monitor_status(
    ctx: &Context,
    units: &[String],
    systemd: &(impl SystemdManager + ?Sized),
) {
    let mut stream = get_status_stream(units, systemd).await.unwrap();
    while let Some(status) = stream.next().await {
        ctx.set_activity(status.map(ActivityData::playing));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systemd::MockSystemdManager;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_update_activity_stream() {
        let (tx, rx) = broadcast::channel::<(&str, &str)>(3);
        let mut manager = MockSystemdManager::new();
        manager
            .expect_active_state_stream()
            .times(2)
            .returning(move |unit| {
                let unit = unit.to_string();
                let mut rx = rx.resubscribe();
                Ok(Box::pin(stream! {
                    loop {
                        let (target, status) = rx.recv().await.unwrap();
                        if target == unit {
                            yield Ok(status.to_string())
                        }
                    }
                }))
            });

        let units = vec!["a.service".to_string(), "b.service".to_string()];
        let mut stream = get_status_stream(units.as_slice(), &manager).await.unwrap();
        tx.send(("a.service", "active")).unwrap();
        assert_eq!(stream.next().await.unwrap(), Some("a.service".to_string()));
        tx.send(("b.service", "active")).unwrap();
        assert_eq!(
            stream.next().await.unwrap(),
            Some("a.service, b.service".to_string())
        );
        tx.send(("a.service", "deactivating")).unwrap();
        assert_eq!(stream.next().await.unwrap(), Some("b.service".to_string()));
    }
}
