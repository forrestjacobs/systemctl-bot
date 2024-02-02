use crate::systemd_status::SystemdStatusManager;
use crate::units::{get_units_with_status_permissions, Unit};
use futures::future::join_all;
use futures::StreamExt;
use indexmap::IndexMap;
use serenity::model::gateway::Activity;
use serenity::prelude::Context;
use tokio_stream::StreamMap;
use zbus::PropertyStream;

pub async fn monitor_status(
    units: &IndexMap<String, Unit>,
    ctx: &Context,
    systemd_status_manager: &SystemdStatusManager,
) -> Result<(), zbus::Error> {
    let units: Vec<&str> = get_units_with_status_permissions(units).collect();
    let streams = units
        .iter()
        .map(|name| systemd_status_manager.status_stream(name));
    let streams = join_all(streams)
        .await
        .into_iter()
        .collect::<Result<Vec<PropertyStream<String>>, zbus::Error>>()?;
    let mut stream = units
        .iter()
        .map(|&name| name)
        .zip(streams)
        .collect::<StreamMap<&str, PropertyStream<String>>>();
    while stream.next().await.is_some() {
        let active_units: Vec<&str> = systemd_status_manager
            .statuses(units.iter().map(|&name| name))
            .await
            .filter(|(_, status)| status.as_ref().map_or(false, |status| status == "active"))
            .map(|(unit, _)| unit)
            .collect();

        if active_units.is_empty() {
            ctx.reset_presence().await;
        } else {
            let activity = Activity::playing(active_units.join(", "));
            ctx.set_activity(activity).await;
        }
    }
    Ok(())
}
