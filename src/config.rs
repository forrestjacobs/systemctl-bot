use std::fs;

use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Service {
    pub name: String,
    pub unit: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub application_id: u64,
    pub discord_token: String,
    pub public_key: String,
    pub guild_id: u64,
    pub services: Vec<Service>,
}

pub fn get_config() -> Config {
    // TODO Better error messaging
    // TODO Take path to config file as command line argument
    let config_string = fs::read_to_string("/etc/systemctl-bot/config.toml")
        .or(fs::read_to_string("./config.toml"))
        .expect("Expected config.toml in /etc/systemctl-bot");
    let config: Config = toml::from_str(config_string.as_str()).unwrap();
    config
}
