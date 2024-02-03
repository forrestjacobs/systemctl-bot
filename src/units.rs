use bitflags::bitflags;
use indexmap::IndexMap;
use serde::{self, Deserialize, Deserializer};

bitflags! {
    #[derive(Deserialize, Clone, Copy)]
    #[serde(rename_all = "snake_case")]
    pub struct UnitPermissions: u32 {
        const Start = 0b00000001;
        const Stop = 0b00000010;
        const Status = 0b00000100;
    }
}

#[derive(Deserialize)]
pub struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    pub name: String,
    pub permissions: UnitPermissions,
}

pub fn get_units_with_permissions<'a>(
    units: &'a IndexMap<String, Unit>,
    permissions: UnitPermissions,
) -> impl Iterator<Item = &'a str> {
    units
        .iter()
        .filter(move |(_, unit)| unit.permissions.contains(permissions))
        .map(|(name, _)| name.as_str())
}

pub fn get_units_with_status_permissions(
    units: &IndexMap<String, Unit>,
) -> impl Iterator<Item = &str> {
    get_units_with_permissions(units, UnitPermissions::Status)
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
