use std::fs;

use indexmap::IndexMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct ServiceToml {
    name: String,
    unit: String,
}

#[derive(Deserialize)]
struct ConfigToml {
    application_id: u64,
    discord_token: String,
    guild_id: u64,
    services: Vec<ServiceToml>,
}

pub struct Service {
    pub unit: String,
}

pub struct Config {
    pub application_id: u64,
    pub discord_token: String,
    pub guild_id: u64,
    pub services: IndexMap<String, Service>,
}

pub fn get_config() -> Config {
    // TODO Better error messaging
    // TODO Take path to config file as command line argument
    let config_toml_string = fs::read_to_string("/etc/systemctl-bot/config.toml").expect("Expected config.toml in /etc/systemctl-bot");
    let config_toml: ConfigToml = toml::from_str(config_toml_string.as_str()).unwrap();

    let mut services = IndexMap::new();
    for service in config_toml.services {
        services.insert(service.name, Service { unit: service.unit });
    }

    Config {
        application_id: config_toml.application_id,
        discord_token: config_toml.discord_token,
        guild_id: config_toml.guild_id,
        services,
    }
}
