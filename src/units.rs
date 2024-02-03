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

pub type Units = IndexMap<String, Unit>;

pub trait UnitsTrait {
    fn with_permissions<'a>(
        &'a self,
        permissions: UnitPermissions,
    ) -> impl Iterator<Item = &'a str>;
}

impl UnitsTrait for Units {
    fn with_permissions<'a>(
        &'a self,
        permissions: UnitPermissions,
    ) -> impl Iterator<Item = &'a str> {
        self.iter()
            .filter(move |(_, unit)| unit.permissions.contains(permissions))
            .map(|(name, _)| name.as_str())
    }
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
