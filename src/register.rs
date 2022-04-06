mod config;

use serde_json::{json, Value};

fn make_command(name: &str, description: &str, options: Vec<Value>) -> Value {
    json!({
        "name": name,
        "type": 1,
        "description": description,
        "options": options,
    })
}

fn make_service_option(description: &str, services: &Vec<config::Service>) -> Value {
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

fn register_commands(
    config: &config::Config,
) -> Result<reqwest::blocking::Response, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    client
        .put(&format!(
            "https://discord.com/api/v8/applications/{}/guilds/{}/commands",
            config.application_id, config.guild_id
        ))
        .header("Authorization", format!("Bot {}", config.discord_token))
        .json(&make_command(
            "systemctl",
            "Controls services",
            vec![
                make_command(
                    "start",
                    "Starts services",
                    vec![make_service_option(
                        "The service to start",
                        &config.services,
                    )],
                ),
                make_command(
                    "stop",
                    "Stops services",
                    vec![make_service_option("The service to stop", &config.services)],
                ),
            ],
        ))
        .send()
}

fn main() {
    if let Err(e) = register_commands(&config::get_config()) {
        println!("Error: {}", e);
        std::process::exit(1);
    }
}
