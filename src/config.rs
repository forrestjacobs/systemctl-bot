use clap::Parser;
use serde::{self, Deserialize, Deserializer};
use serenity::all::{ApplicationId, GuildId};
use std::collections::HashSet;

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[expect(
    clippy::arbitrary_source_item_ordering,
    reason = "Permissions are intentionally ordered"
)]
pub enum UnitPermission {
    Start,
    Stop,
    Status,
}

#[derive(Clone, Debug, Default, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    Multiple,
    #[default]
    Single,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    pub name: String,
    pub permissions: HashSet<UnitPermission>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SystemctlBotConfig {
    pub application_id: ApplicationId,
    #[serde(default)]
    pub command_type: CommandType,
    pub discord_token: String,
    pub guild_id: GuildId,
    pub units: Vec<Unit>,
}

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "/etc/systemctl-bot.toml")]
    config: String,
}

fn deserialize_unit_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let mut name: String = String::deserialize(deserializer)?;
    if !name.contains('.') {
        name = format!("{name}.service");
    }
    Ok(name)
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
                application_id: ApplicationId::new(88_888_888),
                guild_id: GuildId::new(12_345_678),
                discord_token: "88888888.88888888.88888888".to_owned(),
                command_type: CommandType::Single,
                units: vec![
                    Unit {
                        name: "restartable.service".to_owned(),
                        permissions: HashSet::from([UnitPermission::Start, UnitPermission::Stop]),
                    },
                    Unit {
                        name: "reportable.service".to_owned(),
                        permissions: HashSet::from([UnitPermission::Status]),
                    }
                ]
            }
        );
    }

    // TODO: Test missing
    // TODO: Test defaults for command type
    // TODO: Test non-.service units
    // TODO: Test missing units
}
