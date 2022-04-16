use indexmap::IndexMap;
use serde::{self, Deserialize, Deserializer};
use std::collections::HashSet;
use tokio::fs;

#[derive(Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServicePermission {
    Start,
    Stop,
    Status,
}

#[derive(Deserialize)]
pub struct Service {
    pub name: String,
    pub unit: String,
    pub permissions: HashSet<ServicePermission>,
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
    let original_services: Vec<Service> = Vec::deserialize(deserializer)?;
    let mut services = IndexMap::new();
    for service in original_services {
        services.insert(String::from(&service.name), service);
    }
    Ok(services)
}

pub async fn get_config(path: String) -> Result<Config, Box<dyn std::error::Error>> {
    // TODO Take path to config file as command line argument
    Ok(toml::from_str(fs::read_to_string(path).await?.as_str())?)
}
