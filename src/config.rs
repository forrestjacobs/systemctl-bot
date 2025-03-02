use anyhow::{anyhow, Result};
use clap::Parser;
use config::ConfigError;
use poise::serenity_prelude::{ApplicationId, GuildId};
use serde::{self, Deserialize, Deserializer};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    Status,
    Restart,
}

impl TryFrom<&str> for Command {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self> {
        match value {
            "start" => Ok(Command::Start),
            "stop" => Ok(Command::Stop),
            "status" => Ok(Command::Status),
            "restart" => Ok(Command::Restart),
            _ => Err(anyhow!("Could not parse command {}", value)),
        }
    }
}

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UnitPermission {
    Start,
    Stop,
    Status,
}

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
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

#[derive(Debug, Deserialize)]
struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    name: String,
    permissions: HashSet<UnitPermission>,
}

#[derive(Debug, PartialEq)]
pub struct UnitCollection(HashMap<Command, Vec<String>>);

impl From<HashMap<Command, Vec<String>>> for UnitCollection {
    fn from(value: HashMap<Command, Vec<String>>) -> Self {
        UnitCollection(value)
    }
}

impl UnitCollection {
    pub fn ensure_allowed(&self, unit: &str, command: crate::config::Command) -> Result<()> {
        if self.0[&command].iter().any(|u| u == unit) {
            Ok(())
        } else {
            Err(anyhow!("Command is not allowed"))
        }
    }
}

impl Deref for UnitCollection {
    type Target = HashMap<Command, Vec<String>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub application_id: ApplicationId,
    pub discord_token: String,
    pub guild_id: GuildId,
    #[serde(default)]
    pub command_type: CommandType,
    #[serde(deserialize_with = "deserialize_units")]
    pub units: UnitCollection,
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

fn get_units_with_perms<const N: usize>(
    units: &Vec<Unit>,
    perms: [UnitPermission; N],
) -> Vec<String> {
    let perms = HashSet::from(perms);
    units
        .iter()
        .filter(|u| perms.is_subset(&u.permissions))
        .map(|u| u.name.clone())
        .collect()
}

fn deserialize_units<'de, D>(deserializer: D) -> Result<UnitCollection, D::Error>
where
    D: Deserializer<'de>,
{
    let units: Vec<Unit> = Vec::deserialize(deserializer)?;
    println!("{:?}", &units);
    let units = HashMap::from([
        (
            Command::Start,
            get_units_with_perms(&units, [UnitPermission::Start]),
        ),
        (
            Command::Stop,
            get_units_with_perms(&units, [UnitPermission::Stop]),
        ),
        (
            Command::Restart,
            get_units_with_perms(&units, [UnitPermission::Start, UnitPermission::Stop]),
        ),
        (
            Command::Status,
            get_units_with_perms(&units, [UnitPermission::Status]),
        ),
    ]);
    Ok(UnitCollection::from(units))
}

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "/etc/systemctl-bot.toml")]
    config: String,
}

impl Config {
    pub fn build() -> Result<Self, ConfigError> {
        let args = Args::parse();
        let config = config::Config::builder()
            .add_source(config::File::with_name(&args.config))
            .add_source(config::Environment::with_prefix("SBOT"))
            .build()?
            .try_deserialize()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_from_string() {
        assert_eq!(Command::try_from("start").ok(), Some(Command::Start));
        assert_eq!(Command::try_from("stop").ok(), Some(Command::Stop));
        assert_eq!(Command::try_from("status").ok(), Some(Command::Status));
        assert_eq!(Command::try_from("restart").ok(), Some(Command::Restart));
        assert_eq!(
            Command::try_from("random").map_err(|e| e.to_string()),
            Err("Could not parse command random".to_string())
        );
    }

    #[test]
    fn ensure_allowed() {
        let collection = UnitCollection::from(HashMap::from([(
            Command::Start,
            vec![String::from("start.service")],
        )]));
        assert_eq!(
            collection
                .ensure_allowed("start.service", Command::Start)
                .ok(),
            Some(())
        );
        assert_eq!(
            collection
                .ensure_allowed("random.service", Command::Start)
                .map_err(|e| e.to_string()),
            Err("Command is not allowed".to_string())
        );
    }

    #[test]
    fn parse_config() {
        let config: Config = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
                    application_id = 88888888
                    guild_id = 4444
                    discord_token = "88888888.88888888.88888888"
                    command_type = "multiple"

                    [[units]]
                    name = "all"
                    permissions = ["start", "stop", "status"]

                    [[units]]
                    name = "status"
                    permissions = ["status"]

                    [[units]]
                    name = "start"
                    permissions = ["start"]
                "#,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        let all = String::from("all.service");
        assert_eq!(
            config,
            Config {
                application_id: ApplicationId::new(88888888),
                discord_token: String::from("88888888.88888888.88888888"),
                guild_id: GuildId::new(4444),
                command_type: CommandType::Multiple,
                units: UnitCollection(HashMap::from([
                    (
                        Command::Start,
                        vec![all.clone(), String::from("start.service")],
                    ),
                    (Command::Stop, vec![all.clone()],),
                    (Command::Restart, vec![all.clone()],),
                    (
                        Command::Status,
                        vec![all.clone(), String::from("status.service")],
                    ),
                ])),
            }
        );
    }
}
