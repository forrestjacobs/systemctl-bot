use indexmap::IndexMap;
use itertools::Itertools;
use serde::{self, Deserialize, Deserializer};
use std::collections::HashSet;

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitPermission {
    Start,
    Stop,
    Status,
}

#[derive(Deserialize)]
pub struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    pub name: String,
    pub permissions: HashSet<UnitPermission>,
}

pub fn get_units_with_permissions<'a, I: Iterator<Item = &'a UnitPermission>>(
    units: &'a IndexMap<String, Unit>,
    permissions: I,
) -> impl Iterator<Item = &'a str> {
    let superset = permissions.collect_vec();
    units
        .iter()
        .filter(move |(_, unit)| {
            superset
                .iter()
                .all(|permission| unit.permissions.contains(permission))
        })
        .map(|(name, _)| name.as_str())
}

pub fn get_units_with_status_permissions(
    units: &IndexMap<String, Unit>,
) -> impl Iterator<Item = &str> {
    get_units_with_permissions(units, [UnitPermission::Status].iter())
}

fn deserialize_unit_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let mut name: String = String::deserialize(deserializer)?;
    if !name.contains('.') {
        name = format!("{}.service", name);
    }
    Ok(name)
}
