use crate::units::{UnitPermissions, Units};
use config::Config;
use indexmap::IndexMap;
use serde::{self, Deserialize, Deserializer};
use std::collections::HashSet;

#[derive(Debug, Deserialize, PartialEq)]
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

#[derive(Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
enum InternalUnitPermission {
    Start,
    Stop,
    Status,
}

impl InternalUnitPermission {
    fn to_permissions(&self) -> UnitPermissions {
        match self {
            InternalUnitPermission::Start => UnitPermissions::Start,
            InternalUnitPermission::Stop => UnitPermissions::Stop,
            InternalUnitPermission::Status => UnitPermissions::Status,
        }
    }
}

#[derive(Debug, Deserialize)]
struct InternalUnit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    name: String,
    permissions: HashSet<InternalUnitPermission>,
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

#[derive(Debug, Deserialize, PartialEq)]
pub struct SystemctlBotConfig {
    pub application_id: u64,
    pub discord_token: String,
    pub guild_id: u64,
    #[serde(default)]
    pub command_type: CommandType,
    #[serde(deserialize_with = "deserialize_units")]
    pub units: Units,
}

fn deserialize_units<'de, D>(deserializer: D) -> Result<Units, D::Error>
where
    D: Deserializer<'de>,
{
    let original_units: Vec<InternalUnit> = Vec::deserialize(deserializer)?;
    let mut units = IndexMap::new();
    for unit in original_units {
        units.insert(
            String::from(&unit.name),
            unit.permissions
                .into_iter()
                .fold(UnitPermissions::empty(), |acc, p| acc | p.to_permissions()),
        );
    }
    Ok(units)
}

pub fn get_config(path: String) -> Result<SystemctlBotConfig, Box<dyn std::error::Error>> {
    Ok(Config::builder()
        .add_source(config::File::with_name(&path))
        .add_source(config::Environment::with_prefix("SBOT"))
        .build()?
        .try_deserialize()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::FileFormat;

    #[test]
    fn deserialize_config() {
        let toml = r#"
        application_id = 88888888
        guild_id = 99999999
        discord_token = "88888888.88888888.88888888"

        [[units]]
        name = "minecraft"
        permissions = ["start", "stop", "status"]

        [[units]]
        name = "terraria"
        permissions = ["status"]
        "#;
        let config: SystemctlBotConfig = Config::builder()
            .add_source(config::File::from_str(toml, FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert_eq!(
            config,
            SystemctlBotConfig {
                application_id: 88888888,
                discord_token: "88888888.88888888.88888888".to_string(),
                guild_id: 99999999,
                command_type: CommandType::Single,
                units: Units::from([
                    (
                        "minecraft.service".to_string(),
                        UnitPermissions::Start | UnitPermissions::Stop | UnitPermissions::Status
                    ),
                    ("terraria.service".to_string(), UnitPermissions::Status),
                ])
            }
        )
    }
}
