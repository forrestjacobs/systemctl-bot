use std::{env, fs};

use indexmap::IndexMap;

use serde::Deserialize;

#[derive(Deserialize)]
struct ServiceToml {
    name: String,
    unit: String,
}

#[derive(Deserialize)]
struct ConfigToml {
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

fn get_env_var(name: &str) -> String {
    env::var("APPLICATION_ID").expect(format!("Expected environment variable {}", name).as_str())
}

fn get_env_var_u64(name: &str) -> u64 {
    get_env_var(name)
        .parse()
        .expect(format!("Expected {} to be an unsigned 64-bit number", name).as_str())
}

pub fn get_config() -> Config {
    // TODO Better error messaging
    // TODO Take path to config file as command line argument
    let config_toml_string = fs::read_to_string("config.toml").expect("Expected config.toml");
    let config_toml: ConfigToml = toml::from_str(config_toml_string.as_str()).unwrap();

    let mut services = IndexMap::new();
    for service in config_toml.services {
        services.insert(service.name, Service { unit: service.unit });
    }

    // TODO let these come from toml or command line argument
    let application_id = get_env_var_u64("APPLICATION_ID");
    let discord_token = get_env_var("DISCORD_TOKEN");
    let guild_id = get_env_var_u64("GUILD_ID");

    Config {
        application_id,
        discord_token,
        guild_id,
        services,
    }
}
