use crate::systemd_status::SystemdStatusManager;
use crate::units::{UnitPermissions, Units, UnitsTrait};
use futures::future::join_all;
use futures::StreamExt;
use itertools::Itertools;
use serenity::gateway::ActivityData;
use serenity::prelude::Context;
use tokio_stream::StreamMap;
use zbus::PropertyStream;

pub async fn monitor_status(
    units: &Units,
    ctx: &Context,
    systemd_status_manager: &SystemdStatusManager,
) -> Result<(), zbus::Error> {
    let units = units
        .with_permissions(UnitPermissions::Status)
        .collect_vec();
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
        let active_units = systemd_status_manager
            .statuses(units.iter().map(|&name| name))
            .await
            .filter(|(_, status)| status.as_ref().map_or(false, |status| status == "active"))
            .map(|(unit, _)| unit)
            .collect_vec();

        if active_units.is_empty() {
            ctx.reset_presence();
        } else {
            let activity = ActivityData::playing(active_units.join(", "));
            ctx.set_activity(Some(activity));
        }
    }
    Ok(())
}
