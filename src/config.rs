use std::fs;

use indexmap::IndexMap;

use serde::{self, Deserialize, Deserializer};

#[derive(Deserialize)]
struct ServiceToml {
    name: String,
    unit: String,
}

pub struct Service {
    pub unit: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub application_id: u64,
    pub discord_token: String,
    pub guild_id: u64,
    #[serde(deserialize_with = "deserialize_services")]
    pub services: IndexMap<String, Service>,
}

fn deserialize_services<'de, D>(deserializer: D) -> Result<IndexMap<String, Service>, D::Error>
where
    D: Deserializer<'de>,
{
    let original_services: Vec<ServiceToml> = Vec::deserialize(deserializer)?;
    let mut services = IndexMap::new();
    for service in original_services {
        services.insert(service.name, Service { unit: service.unit });
    }
    Ok(services)
}

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    // TODO Take path to config file as command line argument
    Ok(toml::from_str(
        fs::read_to_string("/etc/systemctl-bot/config.toml")?.as_str(),
    )?)
}
