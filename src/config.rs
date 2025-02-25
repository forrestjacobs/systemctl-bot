use clap::Parser;
use poise::serenity_prelude::{ApplicationId, GuildId};
use serde::{self, Deserialize, Deserializer};
use shaku::{Component, Interface, Module, ModuleBuildContext};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

#[derive(Hash, PartialEq, Eq)]
pub enum Command {
    Start,
    Stop,
    Status,
    Restart,
}

#[derive(Debug)]
pub struct CommandParseError;

impl Display for CommandParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Could not parse command")
    }
}

impl Error for CommandParseError {}

impl TryFrom<&str> for Command {
    type Error = CommandParseError;
    fn try_from(value: &str) -> Result<Self, CommandParseError> {
        match value {
            "start" => Ok(Command::Start),
            "stop" => Ok(Command::Stop),
            "status" => Ok(Command::Status),
            "restart" => Ok(Command::Restart),
            _ => Err(CommandParseError),
        }
    }
}

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UnitPermission {
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
struct Unit {
    #[serde(deserialize_with = "deserialize_unit_name")]
    name: String,
    permissions: HashSet<UnitPermission>,
}

#[derive(Deserialize)]
pub struct SystemctlBotConfig {
    pub application_id: ApplicationId,
    pub discord_token: String,
    pub guild_id: GuildId,
    #[serde(default)]
    pub command_type: CommandType,
    #[serde(deserialize_with = "deserialize_units")]
    pub units: HashMap<Command, Vec<String>>,
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
        .filter(|u| u.permissions.is_subset(&perms))
        .map(|u| u.name.clone())
        .collect()
}

fn deserialize_units<'de, D>(deserializer: D) -> Result<HashMap<Command, Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let units: Vec<Unit> = Vec::deserialize(deserializer)?;
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
    Ok(units)
}

pub trait Config: Interface {
    fn get(&self) -> &SystemctlBotConfig;
}

impl Deref for dyn Config {
    type Target = SystemctlBotConfig;
    fn deref(&self) -> &SystemctlBotConfig {
        self.get()
    }
}

pub struct ConfigImpl(SystemctlBotConfig);
impl Config for ConfigImpl {
    fn get(&self) -> &SystemctlBotConfig {
        &self.0
    }
}

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "/etc/systemctl-bot.toml")]
    config: String,
}

impl<M: Module> Component<M> for ConfigImpl {
    type Interface = dyn Config;
    type Parameters = ();

    fn build(_context: &mut ModuleBuildContext<M>, _params: ()) -> Box<dyn Config> {
        let args = Args::parse();
        let config = config::Config::builder()
            .add_source(config::File::with_name(&args.config))
            .add_source(config::Environment::with_prefix("SBOT"))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        Box::new(ConfigImpl(config))
    }
}
