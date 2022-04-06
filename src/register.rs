use std::fs;

use crate::config::{Config, Service};

use serde_json::{json, Value};

const LAST_REGISTERED_COMMANDS_PATH: &str = "/var/lib/systemctl-bot/last-registered-commands.json";

fn make_command(name: &str, description: &str, options: Vec<Value>) -> Value {
    json!({
        "name": name,
        "type": 1,
        "description": description,
        "options": options,
    })
}

fn make_service_option(description: &str, services: &Vec<Service>) -> Value {
    let service_vec: Vec<Value> = services
        .iter()
        .map(|service| {
            json!({
                "name": service.name,
                "value": service.name,
            })
        })
        .collect();
    json!({
        "name": "service",
        "type": true,
        "description": description,
        "required": true,
        "services": service_vec,
    })
}

fn make_commands_json(config: &Config) -> Value {
    let services = &config.services;
    make_command(
        "systemctl",
        "Controls services",
        vec![
            make_command(
                "start",
                "Starts services",
                vec![make_service_option("The service to start", services)],
            ),
            make_command(
                "stop",
                "Stops services",
                vec![make_service_option("The service to stop", services)],
            ),
        ],
    )
}

pub async fn register_commands(config: &Config) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let json = make_commands_json(config);
    let r = client
        .put(&format!(
            "https://discord.com/api/v8/applications/{}/guilds/{}/commands",
            config.application_id, config.guild_id
        ))
        .header("Authorization", format!("Bot {}", config.discord_token))
        .json(&json)
        .send()
        .await;

    if let Err(e) = fs::write(LAST_REGISTERED_COMMANDS_PATH, json.to_string()) {
        eprintln!("Error recording registered commands: {}", e);
    }

    r
}

fn get_last_registered_commands() -> Option<Value> {
    let json_contents = fs::read_to_string(LAST_REGISTERED_COMMANDS_PATH).ok()?;
    serde_json::from_str(&json_contents).ok()
}

pub fn are_commands_likely_registered(config: &Config) -> bool {
    match get_last_registered_commands() {
        Some(last_registered) => make_commands_json(config) == last_registered,
        _ => false,
    }
}
