use indexmap::IndexMap;
use serde::{self, Deserialize, Deserializer};
use std::collections::HashSet;
use tokio::fs;

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitPermission {
    Start,
    Stop,
    Status,
}

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Single,
    Multiple,
}

impl Default for CommandType {
    fn default() -> Self {
        CommandType::Single
    }
}

#[derive(Deserialize)]
pub struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    pub name: String,
    pub permissions: HashSet<UnitPermission>,
}

#[derive(Deserialize)]
pub struct Config {
    pub application_id: u64,
    pub discord_token: String,
    pub guild_id: u64,
    #[serde(default)]
    pub command_type: CommandType,
    #[serde(deserialize_with = "deserialize_units")]
    pub units: IndexMap<String, Unit>,
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

fn deserialize_units<'de, D>(deserializer: D) -> Result<IndexMap<String, Unit>, D::Error>
where
    D: Deserializer<'de>,
{
    let original_units: Vec<Unit> = Vec::deserialize(deserializer)?;
    let mut units = IndexMap::new();
    for unit in original_units {
        units.insert(String::from(&unit.name), unit);
    }
    Ok(units)
}

pub async fn get_config(path: String) -> Result<Config, Box<dyn std::error::Error>> {
    Ok(toml::from_str(fs::read_to_string(path).await?.as_str())?)
}
