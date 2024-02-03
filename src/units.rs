use indexmap::IndexMap;
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

pub fn get_units_with_permissions<const N: usize>(
    units: &IndexMap<String, Unit>,
    permissions: [UnitPermission; N],
) -> impl Iterator<Item = &str> {
    let subset = HashSet::from(permissions);
    units
        .iter()
        .filter(move |(_, unit)| unit.permissions.is_superset(&subset))
        .map(|(name, _)| name.as_str())
}

pub fn get_units_with_status_permissions(
    units: &IndexMap<String, Unit>,
) -> impl Iterator<Item = &str> {
    get_units_with_permissions(units, [UnitPermission::Status])
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
