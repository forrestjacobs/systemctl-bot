use clap::Parser;
use serde::{self, Deserialize, Deserializer};
use serenity::all::{ApplicationId, GuildId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    Restart,
}

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UnitPermission {
    Start,
    Stop,
    Status,
}

#[derive(Debug, Deserialize, Default, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    #[default]
    Single,
    Multiple,
}

#[derive(Deserialize)]
struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    name: String,
    permissions: HashSet<UnitPermission>,
}

pub type CommandUnits = HashMap<Command, Vec<String>>;

#[derive(Debug, PartialEq, Eq)]
pub struct UnitCollection {
    pub command_units: CommandUnits,
    pub status_units: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SystemctlBotConfig {
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
    Ok(UnitCollection {
        command_units: HashMap::from([
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
        ]),
        status_units: get_units_with_perms(&units, [UnitPermission::Status]),
    })
}

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "/etc/systemctl-bot.toml")]
    config: String,
}

pub fn build() -> SystemctlBotConfig {
    let args = Args::parse();
    config::Config::builder()
        .add_source(config::File::with_name(&args.config))
        .add_source(config::Environment::with_prefix("SBOT"))
        .build()
        .expect("Unable to read config")
        .try_deserialize()
        .expect("Unable to parse config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_config() {
        let result: SystemctlBotConfig = config::Config::builder()
            .add_source(config::File::from_str(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                name = "restartable"
                permissions = ["start", "stop"]

                [[units]]
                name = "reportable"
                permissions = ["status"]
                "#,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert_eq!(
            result,
            SystemctlBotConfig {
                application_id: ApplicationId::new(88888888),
                guild_id: GuildId::new(12345678),
                discord_token: "88888888.88888888.88888888".to_string(),
                command_type: CommandType::Single,
                units: UnitCollection {
                    command_units: HashMap::from([
                        (Command::Start, vec!["restartable.service".to_string()]),
                        (Command::Stop, vec!["restartable.service".to_string()]),
                        (Command::Restart, vec!["restartable.service".to_string()]),
                    ]),
                    status_units: vec!["reportable.service".to_string()],
                }
            }
        );
    }

    // TODO: Test missing
    // TODO: Test defaults for command type
    // TODO: Test non-.service units
    // TODO: Test missing units
}
