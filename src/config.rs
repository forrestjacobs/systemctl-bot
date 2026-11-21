use clap::Parser;
use config::{builder::DefaultState, ConfigBuilder, ConfigError};
use serde::{self, de, Deserialize, Deserializer};
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
    #[serde(deserialize_with = "deserialize_unit_permissions")]
    pub permissions: HashSet<UnitPermission>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SystemctlBotConfig {
    pub application_id: ApplicationId,
    #[serde(default)]
    pub command_type: CommandType,
    pub discord_token: String,
    pub guild_id: GuildId,
    #[serde(deserialize_with = "deserialize_units")]
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

    // TODO: Validate names the same as systemd
    if name.is_empty() {
        return Err(de::Error::invalid_value(
            de::Unexpected::Str(&name),
            &"a non-empty unit name",
        ));
    }

    if !name.contains('.') {
        name.push_str(".service");
    }
    Ok(name)
}

fn deserialize_unit_permissions<'de, D>(
    deserializer: D,
) -> Result<HashSet<UnitPermission>, D::Error>
where
    D: Deserializer<'de>,
{
    let permissions: HashSet<UnitPermission> = HashSet::deserialize(deserializer)?;
    let len = permissions.len();
    if len == 0 {
        return Err(de::Error::invalid_length(len, &"at least one permission"));
    }
    Ok(permissions)
}

fn deserialize_units<'de, D>(deserializer: D) -> Result<Vec<Unit>, D::Error>
where
    D: Deserializer<'de>,
{
    let units: Vec<Unit> = Vec::deserialize(deserializer)?;
    let len = units.len();
    if len == 0 {
        return Err(de::Error::invalid_length(len, &"at least one unit"));
    }

    let mut visited_units: HashSet<&str> = HashSet::new();
    for unit in &units {
        let name = unit.name.as_str();
        if !visited_units.insert(name) {
            return Err(de::Error::custom(format!(
                "unexpected duplicate unit \"{name}\""
            )));
        }
    }

    Ok(units)
}

fn deserialize(builder: ConfigBuilder<DefaultState>) -> Result<SystemctlBotConfig, ConfigError> {
    let result = builder.build()?.try_deserialize()?;
    Ok(result)
}

pub fn build() -> Result<SystemctlBotConfig, ConfigError> {
    let args = Args::parse();
    let builder = config::Config::builder()
        .add_source(config::File::with_name(&args.config))
        .add_source(config::Environment::with_prefix("SBOT"));
    deserialize(builder)
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    fn test_build(str: &str) -> Result<SystemctlBotConfig, ConfigError> {
        let source = config::File::from_str(str, config::FileFormat::Toml);
        deserialize(config::Config::builder().add_source(source))
    }

    #[test]
    fn parse_basic_config() {
        let result = test_build(
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
        )
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

    #[test]
    fn parse_config_with_command_type() {
        let result = test_build(
            r#"
            application_id = 88888888
            guild_id = 12345678
            command_type = "multiple"
            discord_token = "88888888.88888888.88888888"

            [[units]]
            name = "restartable"
            permissions = ["start", "stop"]
            "#,
        )
        .unwrap();
        assert_eq!(result.command_type, CommandType::Multiple);
    }

    #[test]
    fn parse_config_with_unit_extension() {
        let result = test_build(
            r#"
            application_id = 88888888
            guild_id = 12345678
            discord_token = "88888888.88888888.88888888"

            [[units]]
            name = "test.timer"
            permissions = ["status"]
            "#,
        )
        .unwrap();
        assert_eq!(result.units[0].name, "test.timer");
    }

    #[test]
    fn parse_missing_config() {
        assert_matches!(
            test_build(""),
            Err(error) if error.to_string() == "missing field `application_id`"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"
                "#,
            ),
            Err(error) if error.to_string() == "missing field `units`"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"
                units = []
                "#,
            ),
            Err(error) if error.to_string() == "invalid length 0, expected at least one unit"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                "#,
            ),
            Err(error) if error.to_string() == "missing field `name`"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                name = ""
                permissions = ["status"]
                "#,
            ),
            Err(error) if error.to_string() == "invalid value: string \"\", expected a non-empty unit name"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                name = "reportable"
                permissions = []
                "#,
            ),
            Err(error) if error.to_string() == "invalid length 0, expected at least one permission"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                name = "reportable"
                permissions = ["status"]

                [[units]]
                name = "reportable"
                permissions = ["status"]
                "#,
            ),
            Err(error) if error.to_string() == "unexpected duplicate unit \"reportable.service\""
        );
    }

    #[test]
    fn parse_bad_config() {
        assert_matches!(
            test_build(
                r#"
                application_id = "88888888.88888888.88888888"
                guild_id = "12345678"
                discord_token = 88888888
                "#,
            ),
            Err(error) if error.to_string() == "invalid digit found in string"
        );
        assert_matches!(
            test_build(
                r#"
                application_id = 88888888
                guild_id = 12345678
                discord_token = "88888888.88888888.88888888"

                [[units]]
                name = "test"
                permissions = ["restart"]
                "#,
            ),
            Err(error) if error.to_string() == "enum UnitPermission does not have variant constructor restart"
        );
    }
}
